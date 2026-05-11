package progress

type EventType string

const (
	EventStart    EventType = "start"
	EventProgress EventType = "progress"
	EventComplete EventType = "complete"
	EventError    EventType = "error"
)

type Category string

const (
	CategoryManifest Category = "manifest"
	CategoryJar      Category = "jar"
	CategoryJRE      Category = "jre"
	CategoryExtract  Category = "extract"
	CategoryBackup   Category = "backup"
)

type Event struct {
	Type     EventType `json:"type"`
	Category Category  `json:"category"`
	Message  string    `json:"message,omitempty"`
	Current  int64     `json:"current,omitempty"`
	Total    int64     `json:"total,omitempty"`
	Error    string    `json:"error,omitempty"`
}

type Callback func(Event)
