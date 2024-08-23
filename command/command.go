package command

import (
	"errors"
	"fmt"
	"strings"
)

type Command interface {
	command()
}

type Launch struct {
	Name string
}

func (l Launch) command() {}

func Parse(input string) (Command, error) {
	args := strings.Split(input, " ")
	if len(args) == 0 {
		return nil, nil
	}
	switch args[0] {
	// case "break", "b":
	// 	handler = cmdParseBreak
	// case "continue", "c":
	// 	handler = cmdParseContinue
	// case "evaluate", "eval", "e":
	// 	handler = cmdParseEvaluate
	case "launch", "l":
		return cmdParseLaunch(args[1:])
	// case "pause", "p":
	// 	handler = cmdParsePause
	// case "quit", "q":
	// 	handler = cmdParseQuit
	// case "up":
	// 	handler = cmdParseUp
	// case "down", "dow":
	// 	handler = cmdParseDown
	default:
		return nil, errors.New(fmt.Sprintf("Unknown command: %s", args[0]))
	}
}

func cmdParseLaunch(args []string) (Launch, error) {
	var l Launch
	if len(args) == 0 {
		return l, errors.New("Did not specify a configuration to launch")
	} else if len(args) > 1 {
		return l, errors.New("Too many arguments")
	}
	l.Name = args[0]
	return l, nil
}
