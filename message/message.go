package message

import (
	"github.com/google/go-dap"
	"github.com/raiguard/pesticide/command"
)

type Message interface {
	msg()
}

type (
	Command struct{ Cmd command.Command }
	DapMsg  struct {
		Adapter string
		Msg     dap.Message
	}
	Print struct{ Obj []any }
)

func (c Command) msg() {}
func (d DapMsg) msg()  {}
func (p Print) msg()   {}
