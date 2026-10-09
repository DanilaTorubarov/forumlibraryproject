import { useState, type SubmitEvent } from 'react'
import '../assets/App.css'

function LoginPage () {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [passwordError, setPasswordError] = useState('')
  const [output, setOutput] = useState('')
  async function register(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    if (password.length<15) {
      setPasswordError('Password is too short')
      return
    }
    setPasswordError('')
    const response = await fetch ('/api/login', {
      method: 'POST',
      headers: {'Content-type': 'application/json'},
      body: JSON.stringify ({
        'email': email,
        'password': password
      })
    })
    if (response.ok) {
      setOutput('Sucsessfully logged in')
    } else {
      setOutput('There is Error')
    }
  }
  return (
    <main>
      <h1>Welcome</h1>
      <form onSubmit={register}>
        <input value={email} type="email" placeholder="Email (example@gmail.com)" onChange={(event) => setEmail(event.target.value)} required/>
        <input value={password} type="password" placeholder="Password (minimal length 15)" onChange={(event) => setPassword(event.target.value)} required/>
        <button type="submit">Register</button>
      </form>
      {passwordError && <p role="alert">{passwordError}</p>}
      {output && <p role="alert">{output}</p>}
    </main>
  )
}

export default LoginPage
