// Stock template, deliberately disconnected from the daemon. The Tauri shell
// exposes commands (app_info, java_list) that the UI can wire up later.
function App() {
  return (
    <main className="container">
      <h1>Hestia</h1>
      <p>The desktop shell is scaffolded. The UI is not wired to the daemon yet.</p>
    </main>
  );
}

export default App;
