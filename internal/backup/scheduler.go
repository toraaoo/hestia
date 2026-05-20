package backup

import (
	"fmt"
	"sync"
	"time"

	"github.com/robfig/cron/v3"
	"github.com/toraaoo/hestia/internal/log"
	"github.com/toraaoo/hestia/internal/server"
)

type Schedule struct {
	Cron      string
	Retention RetentionPolicy
}

type ServerState interface {
	IsRunning(serverName string) bool
	GetRCONInfo(serverName string) (port int, password string, enabled bool)
}

type Scheduler struct {
	cron    *cron.Cron
	jobs    map[string]cron.EntryID
	mu      sync.RWMutex
	state   ServerState
	store   *server.Store
	backups *Service
}

func NewScheduler(state ServerState, store *server.Store, backups *Service) *Scheduler {
	return &Scheduler{
		cron:    cron.New(),
		jobs:    make(map[string]cron.EntryID),
		state:   state,
		store:   store,
		backups: backups,
	}
}

func (s *Scheduler) Start() {
	s.cron.Start()
	log.Info("backup scheduler started")
}

func (s *Scheduler) Stop() {
	ctx := s.cron.Stop()
	<-ctx.Done()
	log.Info("backup scheduler stopped")
}

func (s *Scheduler) UpdateSchedule(serverName string, schedule *Schedule) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if entryID, exists := s.jobs[serverName]; exists {
		s.cron.Remove(entryID)
		delete(s.jobs, serverName)
	}

	if schedule == nil || schedule.Cron == "" {
		return nil
	}

	job := &backupJob{
		serverName: serverName,
		schedule:   schedule,
		state:      s.state,
		backups:    s.backups,
	}

	entryID, err := s.cron.AddJob(schedule.Cron, job)
	if err != nil {
		return fmt.Errorf("invalid cron expression: %w", err)
	}

	s.jobs[serverName] = entryID
	log.Info("scheduled backup", "server", serverName, "cron", schedule.Cron)
	return nil
}

func (s *Scheduler) RemoveSchedule(serverName string) {
	s.mu.Lock()
	defer s.mu.Unlock()

	if entryID, exists := s.jobs[serverName]; exists {
		s.cron.Remove(entryID)
		delete(s.jobs, serverName)
		log.Info("removed backup schedule", "server", serverName)
	}
}

func (s *Scheduler) GetNextRun(serverName string) (time.Time, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	entryID, exists := s.jobs[serverName]
	if !exists {
		return time.Time{}, false
	}

	entry := s.cron.Entry(entryID)
	return entry.Next, true
}

func (s *Scheduler) ListSchedules() map[string]time.Time {
	s.mu.RLock()
	defer s.mu.RUnlock()

	result := make(map[string]time.Time)
	for name, entryID := range s.jobs {
		entry := s.cron.Entry(entryID)
		result[name] = entry.Next
	}
	return result
}

type backupJob struct {
	serverName string
	schedule   *Schedule
	state      ServerState
	backups    *Service
}

func (j *backupJob) Run() {
	log.Info("running scheduled backup", "server", j.serverName)

	opts := Options{
		ServerName: j.serverName,
	}

	if j.state.IsRunning(j.serverName) {
		port, password, enabled := j.state.GetRCONInfo(j.serverName)
		if enabled {
			opts.UseRCON = true
			opts.RCONAddr = fmt.Sprintf("localhost:%d", port)
			opts.RCONPass = password
		} else {
			log.Warn("skipping scheduled backup: server running without RCON", "server", j.serverName)
			return
		}
	}

	info, err := j.backups.Create(opts)
	if err != nil {
		log.Error("scheduled backup failed", "server", j.serverName, "error", err)
		return
	}

	log.Info("scheduled backup complete", "server", j.serverName, "backup", info.Name)

	if j.schedule.Retention.KeepLast > 0 || j.schedule.Retention.KeepDays > 0 {
		deleted, err := j.backups.Prune(j.serverName, j.schedule.Retention)
		if err != nil {
			log.Error("prune failed", "server", j.serverName, "error", err)
		} else if len(deleted) > 0 {
			log.Info("pruned old backups", "server", j.serverName, "count", len(deleted))
		}
	}
}

func (s *Scheduler) LoadSchedules() error {
	servers, err := s.store.List()
	if err != nil {
		return err
	}

	for _, name := range servers {
		cfg, err := s.store.LoadConfig(name)
		if err != nil {
			continue
		}

		if !cfg.Backup.Enabled || cfg.Backup.Schedule == "" {
			continue
		}

		schedule := &Schedule{
			Cron: cfg.Backup.Schedule,
			Retention: RetentionPolicy{
				KeepLast:   cfg.Backup.Retention.KeepLast,
				KeepDays:   cfg.Backup.Retention.KeepDays,
				MinBackups: cfg.Backup.Retention.MinBackups,
			},
		}

		if err := s.UpdateSchedule(name, schedule); err != nil {
			log.Error("failed to load backup schedule", "server", name, "error", err)
		}
	}

	return nil
}
