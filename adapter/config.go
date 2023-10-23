package adapter

import "encoding/json"

type Config struct {
	Cmd  *string
	Args json.RawMessage
	Addr *string
}
