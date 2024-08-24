package command

import (
	"errors"
	"fmt"
	"strings"
)

type Command interface {
	command()
}

type Continue struct{}
type Launch struct {
	Name string
}
type Pause struct{}
type Quit struct{}

func (c Continue) command() {}
func (l Launch) command()   {}
func (p Pause) command()    {}
func (q Quit) command()     {}

func Parse(input string) (Command, error) {
	args := strings.Split(input, " ")
	if len(args) == 0 {
		return nil, nil
	}
	switch args[0] {
	// case "break", "b":
	// 	handler = cmdParseBreak
	case "continue", "c":
		return Continue{}, nil
	// case "evaluate", "eval", "e":
	// 	handler = cmdParseEvaluate
	case "launch", "l":
		return cmdParseLaunch(args[1:])
	case "pause", "p":
		return Pause{}, nil
	case "quit", "q":
		return Quit{}, nil
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
