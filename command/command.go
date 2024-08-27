package command

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

type Command interface {
	command()
}

type (
	Break struct {
		File string
		Line int
	}
	Continue struct{}
	Evaluate struct{ Expr string }
	Launch   struct{ Name string }
	Pause    struct{}
	Quit     struct{}
)

func (b Break) command()    {}
func (c Continue) command() {}
func (e Evaluate) command() {}
func (l Launch) command()   {}
func (p Pause) command()    {}
func (q Quit) command()     {}

func Parse(input string) (Command, error) {
	args := strings.Split(input, " ")
	if len(args) == 0 {
		return nil, nil
	}
	switch args[0] {
	case "break", "b":
		return cmdParseBreak(args[1:])
	case "continue", "c":
		return Continue{}, nil
	case "evaluate", "eval", "e":
		return Evaluate{Expr: strings.Join(args[1:], " ")}, nil
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

func cmdParseBreak(args []string) (Break, error) {
	var b Break
	if len(args) != 2 {
		return b, errors.New("syntax: break <filename> <line>")
	}
	filename, err := filepath.Abs(args[0])
	if err != nil {
		return b, err
	}
	f, err := os.Open(filename)
	defer f.Close()
	if err != nil {
		return b, err
	}
	// TODO: Validate that file exists
	b.File = filename
	line, err := strconv.ParseUint(args[1], 0, 32)
	if err != nil {
		return b, err
	}
	b.Line = int(line)
	return b, nil
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
