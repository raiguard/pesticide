package command

import (
	"errors"
	"fmt"
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
		return command, errors.New("Missing command argument")
	}
	switch name {
	case "launch", "l":
		command.Type = CommandLaunch
		command.Data = strings.TrimSpace(args)
		return command, nil
	}
	return command, errors.New(fmt.Sprintf("Unknown command: %s", name))
}
