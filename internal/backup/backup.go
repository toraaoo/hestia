package backup

import (
	"archive/tar"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/toraaoo/hestia/internal/rcon"
	"github.com/toraaoo/hestia/internal/server"
)

type Info struct {
	Name      string    `json:"name"`
	Path      string    `json:"path"`
	Size      int64     `json:"size"`
	CreatedAt time.Time `json:"created_at"`
	WorldName string    `json:"world_name,omitempty"`
	Version   string    `json:"version,omitempty"`
}

type Options struct {
	ServerName string
	UseRCON    bool
	RCONAddr   string
	RCONPass   string
}

type Service struct {
	store    *server.Store
	rconDial RCONDialer
	locks    sync.Map
	now      func() time.Time
}

type RCONDialer interface {
	Dial(addr, password string) (RCONClient, error)
}

type RCONClient interface {
	Execute(command string) (string, error)
	Close() error
}

type defaultRCONDialer struct{}

func (defaultRCONDialer) Dial(addr, password string) (RCONClient, error) {
	return rcon.Dial(addr, password)
}

var openSourceFile = os.Open

func NewService(store *server.Store, rconDial RCONDialer) *Service {
	if rconDial == nil {
		rconDial = defaultRCONDialer{}
	}
	return &Service{store: store, rconDial: rconDial, now: time.Now}
}

func (s *Service) getLock(serverName string) *sync.Mutex {
	v, _ := s.locks.LoadOrStore(serverName, &sync.Mutex{})
	return v.(*sync.Mutex)
}

func (s *Service) Create(opts Options) (*Info, error) {
	mu := s.getLock(opts.ServerName)
	mu.Lock()
	defer mu.Unlock()

	if !s.store.Exists(opts.ServerName) {
		return nil, fmt.Errorf("server %q not found", opts.ServerName)
	}

	if opts.UseRCON {
		return s.createWithRCON(opts)
	}
	return s.createArchive(opts)
}

func (s *Service) createWithRCON(opts Options) (*Info, error) {
	client, err := s.rconDial.Dial(opts.RCONAddr, opts.RCONPass)
	if err != nil {
		return nil, fmt.Errorf("rcon connect: %w", err)
	}
	defer func() {
		_ = client.Close()
	}()

	if _, err := client.Execute("save-off"); err != nil {
		return nil, fmt.Errorf("save-off: %w", err)
	}

	defer func() {
		_, _ = client.Execute("save-on")
	}()

	if _, err := client.Execute("save-all flush"); err != nil {
		return nil, fmt.Errorf("save-all: %w", err)
	}

	time.Sleep(500 * time.Millisecond)

	return s.createArchive(opts)
}

func (s *Service) createArchive(opts Options) (*Info, error) {
	dataDir := s.store.DataDir(opts.ServerName)
	backupDir := s.store.BackupsDir(opts.ServerName)

	if err := os.MkdirAll(backupDir, 0755); err != nil {
		return nil, fmt.Errorf("create backup dir: %w", err)
	}

	cfg, err := s.store.LoadConfig(opts.ServerName)
	if err != nil {
		return nil, fmt.Errorf("load config: %w", err)
	}

	sources, err := backupSources(dataDir, cfg.World.Name)
	if err != nil {
		return nil, err
	}

	now := s.now()
	timestamp := now.Format("20060102-150405")
	filename := fmt.Sprintf("backup-%s.tar.gz", timestamp)
	backupPath := filepath.Join(backupDir, filename)

	if err := createTarGz(backupPath, dataDir, cfg.World.Name, sources); err != nil {
		return nil, fmt.Errorf("create archive: %w", err)
	}

	stat, err := os.Stat(backupPath)
	if err != nil {
		return nil, err
	}

	info := &Info{
		Name:      filename,
		Path:      backupPath,
		Size:      stat.Size(),
		CreatedAt: now,
		WorldName: cfg.World.Name,
		Version:   cfg.Version,
	}

	metaPath := backupPath + ".json"
	metaData, _ := json.MarshalIndent(info, "", "  ")
	_ = os.WriteFile(metaPath, metaData, 0644)

	return info, nil
}

func backupSources(baseDir, worldName string) ([]string, error) {
	worldDir := filepath.Join(baseDir, worldName)
	if _, err := os.Stat(worldDir); os.IsNotExist(err) {
		return nil, fmt.Errorf("world directory %q not found", worldName)
	}

	sources := []string{worldName}
	for _, name := range []string{"server.properties", "plugins", "mods"} {
		path := filepath.Join(baseDir, name)
		if _, err := os.Stat(path); err == nil {
			sources = append(sources, name)
		}
	}

	return sources, nil
}

func shouldSkipBackupPath(relPath, worldName string) bool {
	relPath = archivePath(relPath)
	worldName = archivePath(worldName)

	if path.Base(relPath) != "session.lock" {
		return false
	}

	return strings.HasPrefix(relPath, worldName+"/")
}

func archivePath(name string) string {
	return strings.ReplaceAll(filepath.ToSlash(name), "\\", "/")
}

func createTarGz(dest, baseDir, worldName string, sources []string) error {
	var retErr error

	f, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer func() {
		if err := f.Close(); err != nil && retErr == nil {
			retErr = err
		}
	}()

	gw := gzip.NewWriter(f)
	defer func() {
		if err := gw.Close(); err != nil && retErr == nil {
			retErr = err
		}
	}()

	tw := tar.NewWriter(gw)
	defer func() {
		if err := tw.Close(); err != nil && retErr == nil {
			retErr = err
		}
	}()

	for _, src := range sources {
		srcPath := filepath.Join(baseDir, src)
		err := filepath.Walk(srcPath, func(path string, fi os.FileInfo, err error) error {
			if err != nil {
				return err
			}

			relPath, err := filepath.Rel(baseDir, path)
			if err != nil {
				return err
			}

			if shouldSkipBackupPath(relPath, worldName) {
				return nil
			}

			header, err := tar.FileInfoHeader(fi, "")
			if err != nil {
				return err
			}
			header.Name = archivePath(relPath)

			if fi.Mode()&os.ModeSymlink != 0 {
				link, err := os.Readlink(path)
				if err != nil {
					return err
				}
				header.Linkname = link
			}

			if err := tw.WriteHeader(header); err != nil {
				return err
			}

			if !fi.Mode().IsRegular() {
				return nil
			}

			file, err := openSourceFile(path)
			if err != nil {
				return err
			}

			_, copyErr := io.Copy(tw, file)
			closeErr := file.Close()
			if copyErr != nil {
				return copyErr
			}
			return closeErr
		})
		if err != nil {
			retErr = err
			return retErr
		}
	}

	return retErr
}

func (s *Service) List(serverName string) ([]Info, error) {
	backupDir := s.store.BackupsDir(serverName)
	entries, err := os.ReadDir(backupDir)
	if os.IsNotExist(err) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	var backups []Info
	for _, e := range entries {
		if e.IsDir() || !strings.HasSuffix(e.Name(), ".tar.gz") {
			continue
		}

		metaPath := filepath.Join(backupDir, e.Name()+".json")
		if data, err := os.ReadFile(metaPath); err == nil {
			var info Info
			if json.Unmarshal(data, &info) == nil {
				backups = append(backups, info)
				continue
			}
		}

		fi, err := e.Info()
		if err != nil {
			continue
		}

		backups = append(backups, Info{
			Name:      e.Name(),
			Path:      filepath.Join(backupDir, e.Name()),
			Size:      fi.Size(),
			CreatedAt: fi.ModTime(),
		})
	}

	sort.Slice(backups, func(i, j int) bool {
		return backups[i].CreatedAt.After(backups[j].CreatedAt)
	})

	return backups, nil
}

func (s *Service) Restore(serverName, backupName string) error {
	mu := s.getLock(serverName)
	mu.Lock()
	defer mu.Unlock()

	backupPath := filepath.Join(s.store.BackupsDir(serverName), backupName)
	if _, err := os.Stat(backupPath); os.IsNotExist(err) {
		return fmt.Errorf("backup %q not found", backupName)
	}

	cfg, err := s.store.LoadConfig(serverName)
	if err != nil {
		return fmt.Errorf("load config: %w", err)
	}

	dataDir := s.store.DataDir(serverName)
	restoreTargets := []string{
		filepath.Join(dataDir, cfg.World.Name),
		filepath.Join(dataDir, "server.properties"),
		filepath.Join(dataDir, "plugins"),
		filepath.Join(dataDir, "mods"),
	}

	for _, target := range restoreTargets {
		if err := os.RemoveAll(target); err != nil {
			return fmt.Errorf("remove old backup target %q: %w", filepath.Base(target), err)
		}
	}

	if err := extractTarGz(backupPath, dataDir); err != nil {
		return fmt.Errorf("extract backup: %w", err)
	}

	return nil
}

func extractTarGz(src, dest string) error {
	var retErr error

	f, err := os.Open(src)
	if err != nil {
		return err
	}
	defer func() {
		if err := f.Close(); err != nil && retErr == nil {
			retErr = err
		}
	}()

	gr, err := gzip.NewReader(f)
	if err != nil {
		return err
	}
	defer func() {
		if err := gr.Close(); err != nil && retErr == nil {
			retErr = err
		}
	}()

	tr := tar.NewReader(gr)

	for {
		header, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return err
		}

		target := filepath.Join(dest, header.Name)

		if !strings.HasPrefix(target, filepath.Clean(dest)+string(os.PathSeparator)) {
			return fmt.Errorf("invalid tar path: %s", header.Name)
		}

		switch header.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, os.FileMode(header.Mode)); err != nil {
				return err
			}
		case tar.TypeReg:
			if err := os.MkdirAll(filepath.Dir(target), 0755); err != nil {
				return err
			}
			outFile, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, os.FileMode(header.Mode))
			if err != nil {
				return err
			}

			_, copyErr := io.Copy(outFile, tr)
			closeErr := outFile.Close()
			if copyErr != nil {
				return copyErr
			}
			if closeErr != nil {
				return closeErr
			}
		case tar.TypeSymlink:
			if err := os.MkdirAll(filepath.Dir(target), 0755); err != nil {
				return err
			}
			if err := os.Symlink(header.Linkname, target); err != nil {
				return err
			}
		}
	}

	return retErr
}

func (s *Service) Delete(serverName, backupName string) error {
	backupPath := filepath.Join(s.store.BackupsDir(serverName), backupName)
	if _, err := os.Stat(backupPath); os.IsNotExist(err) {
		return fmt.Errorf("backup %q not found", backupName)
	}

	if err := os.Remove(backupPath); err != nil {
		return err
	}

	metaPath := backupPath + ".json"
	_ = os.Remove(metaPath)

	return nil
}
