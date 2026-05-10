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
	"github.com/toraaoo/hestia/internal/jre"
	"github.com/toraaoo/hestia/internal/server"
)

type Process struct {
	Name   string
	Config *server.Config
	State  State
	PID    int

	cmd    *exec.Cmd
	stdin  io.WriteCloser
	ring   *RingBuffer
	logw   *LogWriter
	mu     sync.RWMutex
	cancel context.CancelFunc
}

type Manager struct {
	procs map[string]*Process
	mu    sync.RWMutex
}

func NewManager() *Manager {
	return &Manager{procs: make(map[string]*Process)}
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
	cfg, err := server.LoadConfig(name)
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
	}()

	javaVersion, err := jar.VanillaProvider{}.GetJavaVersion(p.Config.Version)
	if err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: get java version: %v\n", err))
		return
	}

	javaPath, err := jre.GetJRE(javaVersion)
	if err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: get jre: %v\n", err))
		return
	}

	jarPath := server.JarPath(p.Name)
	if _, err := os.Stat(jarPath); os.IsNotExist(err) {
		p.ring.Write(fmt.Sprintf("Downloading server jar %s...\n", p.Config.Version))
		provider := jar.VanillaProvider{}
		if err := provider.DownloadServer(p.Config.Version, jarPath); err != nil {
			p.ring.Write(fmt.Sprintf("ERROR: download server: %v\n", err))
			return
		}
	}

	serverDir := server.ServerDir(p.Name)
	eulaPath := serverDir + "/eula.txt"
	os.WriteFile(eulaPath, []byte("eula=true\n"), 0644)

	ctx, cancel := context.WithCancel(context.Background())
	p.cancel = cancel

	p.cmd = exec.CommandContext(ctx, javaPath,
		"-Xmx"+p.Config.Memory,
		"-Xms"+p.Config.Memory,
		"-jar", "server.jar",
		"nogui",
	)
	p.cmd.Dir = serverDir

	p.logw, err = NewLogWriter(serverDir, p.ring)
	if err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: create log writer: %v\n", err))
		return
	}
	defer p.logw.Close()

	p.stdin, _ = p.cmd.StdinPipe()
	stdout, _ := p.cmd.StdoutPipe()
	stderr, _ := p.cmd.StderrPipe()

	if err := p.cmd.Start(); err != nil {
		p.ring.Write(fmt.Sprintf("ERROR: start server: %v\n", err))
		return
	}

	p.mu.Lock()
	p.PID = p.cmd.Process.Pid
	p.State = StateRunning
	p.mu.Unlock()

	go ScanLines(stdout, p.logw)
	go ScanLines(stderr, p.logw)

	p.cmd.Wait()
}

func (p *Process) stop() error {
	p.mu.Lock()
	if p.State != StateRunning {
		p.mu.Unlock()
		return fmt.Errorf("server not running")
	}
	p.State = StateStopping
	p.mu.Unlock()

	p.sendCommand("stop")

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
