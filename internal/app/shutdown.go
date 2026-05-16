package app

import "sync"

type Shutdown struct {
	ch   chan struct{}
	once sync.Once
}

func NewShutdown() *Shutdown {
	return &Shutdown{ch: make(chan struct{})}
}

func (s *Shutdown) Trigger() bool {
	triggered := false
	s.once.Do(func() {
		close(s.ch)
		triggered = true
	})
	return triggered
}

func (s *Shutdown) Done() <-chan struct{} {
	return s.ch
}
