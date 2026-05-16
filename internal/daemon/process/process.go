package process

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"sync"
	"time"

	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/log"
	"github.com/toraaoo/hestia/internal/progress"
	"github.com/toraaoo/hestia/internal/server"
)

type ServerStore interface {
	LoadConfig(name string) (*server.Config, error)
	ServerDir(name string) string
	JarPath(name string) string
}

type JREManager interface {
	Get(majorVersion int, cb progress.Callback) (string, error)
}

type JarRegistry interface {
	GetProvider(name string) (jar.Loader, error)
}

type Process struct {
	Name   string
	Config *server.Config
	State  State
	PID    int
	store  ServerStore
	jre    JREManager
	jars   JarRegistry

	cmd    *exec.Cmd
	stdin  io.WriteCloser
	ring   *RingBuffer
	logw   *LogWriter
	mu     sync.RWMutex
	cancel context.CancelFunc
}

type Manager struct {
	procs map[string]*Process
	store ServerStore
	jre   JREManager
	jars  JarRegistry
	mu    sync.RWMutex
}

func NewManager(store ServerStore, jre JREManager, jars JarRegistry) *Manager {
	return &Manager{
		procs: make(map[string]*Process),
		store: store,
		jre:   jre,
		jars:  jars,
	}
}

func (m *Manager) Get(name string) *Process {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.procs[name]
}

func (m *Manager) All() []*Process {
	m.mu.RLock()
	defer m.mu.RUnlock()
	result := make([]*Process, 0, len(m.procs))
	for _, p := range m.procs {
		result = append(result, p)
	}
	return result
}

func (m *Manager) Start(name string) error {
	cfg, err := m.store.LoadConfig(name)
	if err != nil {
		return err
	}

	m.mu.Lock()
	if p, exists := m.procs[name]; exists && p.State != StateStopped {
		m.mu.Unlock()
		return fmt.Errorf("server %s already running", name)
	}

	proc := &Process{
		Name:   name,
		Config: cfg,
		State:  StateStarting,
		store:  m.store,
		jre:    m.jre,
		jars:   m.jars,
		ring:   NewRingBuffer(1000),
	}
	m.procs[name] = proc
	m.mu.Unlock()

	go proc.run()
	return nil
}

func (m *Manager) Stop(name string) error {
	m.mu.RLock()
	proc := m.procs[name]
	m.mu.RUnlock()

	if proc == nil {
		return fmt.Errorf("server %s not running", name)
	}
	return proc.stop()
}

// StopAll stops all running processes concurrently and waits for them to finish.
func (m *Manager) StopAll() {
	m.mu.RLock()
	var names []string
	for name, p := range m.procs {
		if p.State != StateStopped {
			names = append(names, name)
		}
	}
	m.mu.RUnlock()

	var wg sync.WaitGroup
	for _, name := range names {
		wg.Add(1)
		go func(n string) {
			defer wg.Done()
			if err := m.Stop(n); err != nil {
				log.Warn("stop server on shutdown", "server", n, "err", err)
			}
		}(name)
	}
	wg.Wait()
}

func (m *Manager) SendCommand(name, cmd string) error {
	m.mu.RLock()
	proc := m.procs[name]
	m.mu.RUnlock()

	if proc == nil || proc.State != StateRunning {
		return fmt.Errorf("server %s not running", name)
	}
	return proc.sendCommand(cmd)
}

func (m *Manager) Logs(name string, n int) ([]LogLine, error) {
	m.mu.RLock()
	proc := m.procs[name]
	m.mu.RUnlock()

	if proc == nil {
		return nil, fmt.Errorf("server %s not found", name)
	}
	return proc.ring.Last(n), nil
}

func (m *Manager) Subscribe(name string) (chan LogLine, error) {
	m.mu.RLock()
	proc := m.procs[name]
	m.mu.RUnlock()

	if proc == nil || proc.logw == nil {
		return nil, fmt.Errorf("server %s not running", name)
	}
	return proc.logw.Subscribe(), nil
}

func (m *Manager) Unsubscribe(name string, ch chan LogLine) {
	m.mu.RLock()
	proc := m.procs[name]
	m.mu.RUnlock()

	if proc != nil && proc.logw != nil {
		proc.logw.Unsubscribe(ch)
	}
}

func (p *Process) run() {
	defer func() {
		p.mu.Lock()
		p.State = StateStopped
		p.mu.Unlock()
		log.Info("server stopped", "server", p.Name)
	}()

	provider, err := p.jars.GetProvider(p.Config.Jar)
	if err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: unknown jar provider %q: %v\n", p.Config.Jar, err))
		return
	}

	javaVersion, err := provider.GetJavaVersion(p.Config.Version)
	if err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: get java version: %v\n", err))
		return
	}

	javaPath, err := p.jre.Get(javaVersion, nil)
	if err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: get jre: %v\n", err))
		return
	}

	jarPath := p.store.JarPath(p.Name)
	if _, err := os.Stat(jarPath); os.IsNotExist(err) {
		p.ring.Write(fmt.Sprintf("Downloading server jar %s...\n", p.Config.Version))
		if err := provider.DownloadServer(p.Config.Version, jarPath, nil); err != nil {
			p.ring.Write(fmt.Sprintf("ERROR: download server: %v\n", err))
			return
		}
	}

	serverDir := p.store.ServerDir(p.Name)
	eulaPath := serverDir + "/eula.txt"
	if err := os.WriteFile(eulaPath, []byte("eula=true\n"), 0644); err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: write eula: %v\n", err))
		return
	}

	ctx, cancel := context.WithCancel(context.Background())
	p.cancel = cancel

	p.cmd = exec.CommandContext(ctx, javaPath,
		"-Xmx"+p.Config.Memory,
		"-Xms"+p.Config.Memory,
		"-jar", "server.jar",
		"nogui",
	)
	p.cmd.Dir = serverDir

	logw, err := NewLogWriter(serverDir, p.ring)
	if err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: create log writer: %v\n", err))
		return
	}
	p.logw = logw

	stdin, err := p.cmd.StdinPipe()
	if err != nil {
		_ = logw.Close()
		p.ring.Write(fmt.Sprintf("ERROR: stdin pipe: %v\n", err))
		return
	}
	p.stdin = stdin

	stdout, err := p.cmd.StdoutPipe()
	if err != nil {
		_ = logw.Close()
		p.ring.Write(fmt.Sprintf("ERROR: stdout pipe: %v\n", err))
		return
	}

	stderr, err := p.cmd.StderrPipe()
	if err != nil {
		_ = logw.Close()
		p.ring.Write(fmt.Sprintf("ERROR: stderr pipe: %v\n", err))
		return
	}

	if err := p.cmd.Start(); err != nil {
		_ = logw.Close()
		p.ring.Write(fmt.Sprintf("ERROR: start server: %v\n", err))
		return
	}

	p.mu.Lock()
	p.PID = p.cmd.Process.Pid
	p.State = StateRunning
	p.mu.Unlock()
	log.Info("server started", "server", p.Name, "pid", p.PID)

	var wg sync.WaitGroup
	wg.Add(2)
	go func() { defer wg.Done(); ScanLines(stdout, logw) }()
	go func() { defer wg.Done(); ScanLines(stderr, logw) }()

	_ = p.cmd.Wait()
	wg.Wait()
	_ = logw.Close()
	p.logw = nil
}

func (p *Process) stop() error {
	p.mu.Lock()
	if p.State != StateRunning {
		p.mu.Unlock()
		return fmt.Errorf("server not running")
	}
	p.State = StateStopping
	p.mu.Unlock()

	_ = p.sendCommand("stop")

	done := make(chan struct{})
	go func() {
		for {
			p.mu.RLock()
			state := p.State
			p.mu.RUnlock()
			if state == StateStopped {
				close(done)
				return
			}
			time.Sleep(100 * time.Millisecond)
		}
	}()

	select {
	case <-done:
		return nil
	case <-time.After(30 * time.Second):
		if p.cancel != nil {
			p.cancel()
		}
		return nil
	}
}

func (p *Process) sendCommand(cmd string) error {
	if p.stdin == nil {
		return fmt.Errorf("stdin not available")
	}
	_, err := p.stdin.Write([]byte(cmd + "\n"))
	return err
}

func (p *Process) GetState() State {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.State
}
