package backup

import (
	"os"
	"time"
)

type RetentionPolicy struct {
	KeepLast   int `toml:"keep_last" json:"keep_last"`
	KeepDays   int `toml:"keep_days" json:"keep_days"`
	MinBackups int `toml:"min_backups" json:"min_backups"`
}

func DefaultRetention() RetentionPolicy {
	return RetentionPolicy{
		KeepLast:   10,
		KeepDays:   7,
		MinBackups: 3,
	}
}

func (p RetentionPolicy) Apply(backups []Info) []Info {
	if len(backups) == 0 {
		return nil
	}

	keep := make(map[string]bool)
	cutoff := time.Now().AddDate(0, 0, -p.KeepDays)

	for i, b := range backups {
		if p.KeepLast > 0 && i < p.KeepLast {
			keep[b.Name] = true
		}
		if p.KeepDays > 0 && b.CreatedAt.After(cutoff) {
			keep[b.Name] = true
		}
	}

	var toDelete []Info
	for i, b := range backups {
		if !keep[b.Name] && i >= p.MinBackups {
			toDelete = append(toDelete, b)
		}
	}

	return toDelete
}

func (s *Service) Prune(serverName string, policy RetentionPolicy) ([]string, error) {
	backups, err := s.List(serverName)
	if err != nil {
		return nil, err
	}

	toDelete := policy.Apply(backups)
	var deleted []string

	for _, b := range toDelete {
		if err := os.Remove(b.Path); err == nil {
			deleted = append(deleted, b.Name)
			_ = os.Remove(b.Path + ".json")
		}
	}

	return deleted, nil
}
