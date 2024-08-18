package config

import (
	"encoding/json"
	"errors"
	"os"
)

type Config struct {
	Adapters map[string]AdapterConfig
}

type AdapterConfig struct {
	Cmd  *string
	Args json.RawMessage
	Addr *string
}

func New(path string) Config {
	file, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	var config Config
	if err = json.Unmarshal(file, &config); err != nil {
		panic(err)
	}

	if len(config.Adapters) == 0 {
		panic(errors.New("No adapters were specified"))
	}
	for name, adapter := range config.Adapters {
		if adapter.Addr == nil && adapter.Cmd == nil {
			panic(errors.New("Adapters must have an address or command to run"))
		}
		if adapter.Cmd != nil {
			expanded := os.ExpandEnv(*adapter.Cmd)
			adapter.Cmd = &expanded
		}
		config.Adapters[name] = adapter
	}
	return config
}
