package main

import (
	"encoding/json"
	"log"
	"net"
	"os"
	"strings"

	"git.sr.ht/~emersion/go-scfg"
)

// Cmd parses user input and configuration files using the scfg syntax, and
// executes UI or adapter commands.

func cmdReadFile(path string) {
	block, err := scfg.Load(path)
	if err != nil {
		log.Println("Failed to read file", path, ":", err)
		return
	}
	cmdParseBlock(block)
}

func cmdRead(input string) {
	reader := strings.NewReader(input)
	block, err := scfg.Read(reader)
	if err != nil {
		log.Println(err)
		return
	}
	cmdParseBlock(block)
}

func cmdParseBlock(block scfg.Block) {
	for i := 0; i < len(block); i++ {
		cmdParseDirective(block[i])
	}
}

func cmdParseDirective(directive *scfg.Directive) {
	switch directive.Name {
	case "adapter":
		cmdParseAdapter(directive)
	}
}

type adapterConfig struct {
	name  string
	type_ string
	cmd   *string
	addr  *net.TCPAddr
	// Arbitrary key/value pairs
	// args *map[string]interface{}
	args *json.RawMessage
}

func cmdParseAdapter(directive *scfg.Directive) {
	if len(directive.Params) != 1 {
		panic("adapter command must have only one argument")
	}

	cfg := adapterConfig{name: directive.Params[0]}

	for _, child := range directive.Children {
		switch child.Name {
		case "type":
			cfg.type_ = child.Params[0]
		case "cmd":
			expanded := os.ExpandEnv(child.Params[0])
			cfg.cmd = &expanded
		case "addr":
			addr, err := net.ResolveTCPAddr("tcp", child.Params[0])
			if err != nil {
				panic(err)
			}
			cfg.addr = addr
		case "args":
			// TODO: Make this more ergonomic
			value := child.Params[0]
			bytes := make(json.RawMessage, len(value))
			copy(bytes, value)
			cfg.args = &bytes
		default:
		}
	}

	adapterConfigs[directive.Params[0]] = &cfg
}
