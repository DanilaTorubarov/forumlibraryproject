use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use axum::{ Json, Router, http::{ StatusCode }, routing::post, extract::State };
use argon2::{ Argon2, password_hash::{ PasswordHasher, PasswordVerifier, phc::PasswordHash } };
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
    axum::serve(listener, app).await?;
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
    let email = input.email.to_lowercase();
    let login = input.login.to_lowercase();
    let missing_text = [&input.email, &input.name, &input.surname, &input.login]
        .iter()
        .any(|value| value.trim().is_empty());
    if
        !login.chars().all(|ch| { ch.is_ascii_alphanumeric() || matches!(ch, '_' | '!' | '@') }) ||
        !email_address::EmailAddress::is_valid(&email) ||
        missing_text ||
        input.password.chars().count() < 15
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let existing = sqlx
        ::query("SELECT 1 FROM users WHERE email=$1 OR login=$2")
        .bind(&email)
        .bind(&login)
        .fetch_optional(&pool).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match existing {
        Some(_) => {
            return Err(StatusCode::CONFLICT);
        }
        None => {}
    }
    let argon2 = Argon2::default();
    let password_bytes = input.password.as_bytes();
    let password_hash = argon2
        .hash_password(password_bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();
    sqlx
        ::query(
            "INSERT INTO users (name,surname,login,login_displayed,email,password_hash) VALUES ($1,$2,$3,$4,$5,$6)"
        )
        .bind(&input.name)
        .bind(&input.surname)
        .bind(&login)
        .bind(&input.login)
        .bind(&email)
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
    let email = input.email.to_lowercase();
    let row = sqlx
        ::query("SELECT password_hash FROM users WHERE email = $1")
        .bind(&email)
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
    use rand::{ distr::Alphanumeric, RngExt };

    #[tokio::test]
    async fn database_connection_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        Ok(())
    }

    #[tokio::test]
    async fn database_user_login_or_register_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        let random: String = rand
            ::rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let email = format!("test_{random}@example.com").to_lowercase();
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User@".to_owned(),
                login: format!("test_{random}").to_lowercase(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        assert_eq!(result, Ok(StatusCode::CREATED));
        let result = login_user(
            State(pool.clone()),
            Json(Login {
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Ok(StatusCode::OK));
        Ok(())
    }

    #[tokio::test]
    async fn register_empty_fields() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        let random: String = rand
            ::rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let email = format!("test_{random}@example.com").to_lowercase();
        let login = random.to_lowercase();
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "".to_owned(),
                surname: "User".to_owned(),
                login: login.clone(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "".to_owned(),
                login: login.clone(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: "".to_owned(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "".to_owned(),
                surname: "User".to_owned(),
                login: login.clone(),
                email: "".to_owned(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "".to_owned(),
                surname: "User".to_owned(),
                login: login.clone(),
                email: email.clone(),
                password: "".to_owned(),
            })
        ).await;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        Ok(())
    }

    #[tokio::test]
    async fn register_login_characters_fields() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        let random: String = rand
            ::rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let email = format!("test_{random}@example.com").to_lowercase();
        let login = format!("!te_{random}@").to_lowercase();
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: "Test Test".to_owned(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: "Test.Test".to_owned(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: "Test,Test".to_owned(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: login.to_owned(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Ok(StatusCode::CREATED));
        Ok(())
    }

    #[tokio::test]
    async fn register_invalid_email() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        let email: String = rand
            ::rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: email.clone(),
                email: email.clone(),
                password: "a-long-test-password".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        Ok(())
    }

    #[tokio::test]
    async fn register_short_password() -> Result<(), Box<dyn std::error::Error>> {
        let pool = connect().await?;
        let random: String = rand
            ::rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let email = format!("test_{random}@example.com").to_lowercase();
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: random.to_lowercase(),
                email: email.clone(),
                password: "15fifteencharac".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Ok(StatusCode::CREATED));
        let result = register_user(
            State(pool.clone()),
            Json(Registration {
                name: "Test".to_owned(),
                surname: "User".to_owned(),
                login: random.to_lowercase(),
                email: email.clone(),
                password: "14fourteenchar".to_owned(),
            })
        ).await;
        sqlx::query("DELETE FROM users WHERE email= $1").bind(email.clone()).execute(&pool).await?;
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        Ok(())
    }
}
