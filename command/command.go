package command

import (
	"errors"
	"strings"
)

type Command struct {
	Type CommandType
	Data string
}

type CommandType string

const (
	CommandInvalid CommandType = "invalid"
	CommandLaunch  CommandType = "launch"
)

func Parse(input string) (Command, error) {
	command := Command{CommandInvalid, ""}
	name, args, ok := strings.Cut(input, " ")
	if !ok {
		return command, errors.New("Invalid command")
	}
	switch name {
	case "launch":
		command.Type = CommandLaunch
		command.Data = strings.TrimSpace(args)
		return command, nil
	}
	return command, errors.New("Unknown command")
}
