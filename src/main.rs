use serde;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use axum::{ Json, Router, http::{ self, StatusCode }, routing::post, extract::State };
use argon2::{
    Argon2,
    password_hash::{ self, PasswordHasher, PasswordVerifier, phc::PasswordHash },
};
use sqlx::Row;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let pool = connect().await?;
    let app = Router::new()
        .route("/api/register", post(register_user))
        .route("/api/login", post(login_user))
        .fallback_service(ServeDir::new("frontend/dist"))
        .with_state(pool);
    println!("Server running: http://127.0.0.1:8080");
    let _ = axum::serve(listener, app).await;
    Ok(())
}

#[derive(serde::Deserialize)]
struct Registration {
    name: String,
    surname: String,
    login: String,
    email: String,
    password: String,
}

#[derive(serde::Deserialize)]
struct Login {
    email: String,
    password: String,
}

async fn connect() -> Result<sqlx::PgPool, Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&url).await?;
    Ok(pool)
}

async fn register_user(
    State(pool): State<sqlx::PgPool>,
    Json(input): Json<Registration>
) -> Result<StatusCode, StatusCode> {
    let missing_text = [&input.email, &input.name, &input.surname, &input.login]
        .iter()
        .any(|value| value.trim().is_empty());
    if
        !input.login
            .chars()
            .all(|ch| { ch.is_ascii_alphanumeric() || matches!(ch, '_' | '!' | '@') }) ||
        !email_address::EmailAddress::is_valid(&input.email) ||
        missing_text ||
        input.password.chars().count() < 15
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let argon2 = Argon2::default();
    let password_bytes: &[u8] = input.password.as_bytes();
    let password_hash = argon2
        .hash_password(password_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();
    sqlx
        ::query(
            "INSERT INTO users (name,surname,login,email,password_hash) VALUES ($1,$2,$3,$4,$5)"
        )
        .bind(&input.name)
        .bind(&input.surname)
        .bind(&input.login)
        .bind(&input.email)
        .bind(&password_hash)
        .execute(&pool).await
        .map_err(|error| {
            match error {
                sqlx::Error::Database(db) if db.is_unique_violation() => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
    Ok(StatusCode::CREATED)
}

async fn login_user(
    State(pool): State<sqlx::PgPool>,
    Json(input): Json<Login>
) -> Result<StatusCode, StatusCode> {
    let row = sqlx
        ::query("SELECT password_hash FROM users WHERE email = $1")
        .bind(&input.email)
        .fetch_optional(&pool).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(row) = row else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let password_hash: String = row
        .try_get("password_hash")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let parsed_hash = PasswordHash::new(&password_hash).map_err(
        |_| StatusCode::INTERNAL_SERVER_ERROR
    )?;
    if Argon2::default().verify_password(input.password.as_bytes(), &parsed_hash).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn database_connection_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        Ok(())
    }

    #[tokio::test]
    async fn database_user_register_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: "test_user_123".to_owned(),
                email: "test_123@example.com".to_owned(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        assert_eq!(result, Ok(StatusCode::CREATED));
        sqlx
            ::query("DELETE FROM users WHERE email= $1")
            .bind("test_123@example.com")
            .execute(&pool).await?;
        Ok(())
    }

    #[tokio::test]
    async fn database_user_login_works() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
