import { useState, type SubmitEvent } from 'react'
import './assets/App.css'

function App() {
  const [firstName, setFirstName] = useState('')
  const [surname, setSurname] = useState('')
  const [login, setLogin] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [passwordr, setPasswordr] = useState('')
  const [passwordError, setPasswordError] = useState('')
  const [output, setOutput] = useState('')
  async function register(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    if (password !== passwordr) {
      setPasswordError('Passwords do not match')
      return
    }
    if (password.length<15) {
      setPasswordError('Password is too short')
      return
    }
    setPasswordError('')
    const response = await fetch ('/api/register', {
      method: 'POST',
      headers: {'Content-type': 'application/json'},
      body: JSON.stringify ({
        'name': firstName,
        'surname': surname,
        'login': login,
        'email': email,
        'password': password
      })
    })
    if (response.ok) {
      setOutput('Sucsessfully registered')
    } else {
      setOutput('There is Error')
    }
  }
  return (
    <main>
      <h1>Welcome</h1>
      <form onSubmit={register}>
        <input value={firstName} type="text" placeholder="First name" onChange={(event) => setFirstName(event.target.value)} required/>
        <input value={surname} type="text" placeholder="Surname" onChange={(event) => setSurname(event.target.value)} required/>
        <input value={login} type="text" placeholder="Login (numbers, letters, and _!@)" onChange={(event) => setLogin(event.target.value)} required/>
        <input value={email} type="email" placeholder="Email (example@gmail.com)" onChange={(event) => setEmail(event.target.value)} required/>
        <input value={password} type="password" placeholder="Password (minimal length 15)" onChange={(event) => setPassword(event.target.value)} required/>
        <input value={passwordr} type="password" placeholder="Repeat password" onChange={(event) => setPasswordr(event.target.value)} required/>
        <button type="submit">Register</button>
      </form>
      {passwordError && <p role="alert">{passwordError}</p>}
      {output && <p role="alert">{output}</p>}
      <p>Hello, {firstName} {surname}!</p>
    </main>
  )
}


export default App
