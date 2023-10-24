package adapter

import "encoding/json"

type Config struct {
	Cmd  *string         `json:"command"`
	Args json.RawMessage `json:"arguments"`
	Addr *string         `json:"address"`
}
