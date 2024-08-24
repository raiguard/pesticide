package router

import (
	"fmt"

	"github.com/raiguard/pesticide/adapter"
	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/message"
)

type Router struct {
	input  chan message.Message
	output chan message.Message

	config config.Config

	adapters       map[string]*adapter.Adapter
	focusedAdapter *adapter.Adapter
}

func New(input chan message.Message, output chan message.Message, config config.Config) *Router {
	return &Router{
		input:    input,
		output:   output,
		config:   config,
		adapters: map[string]*adapter.Adapter{},
	}
}

func (r *Router) Run() {
	for msg := range r.input {
		switch msg := msg.(type) {
		case message.Command:
			if err := r.handleCommand(msg.Cmd); err != nil {
				r.println(err)
			}
		case message.DapMsg:
			if err := r.handleDAPMessage(msg); err != nil {
				r.println(err)
			}
		}
	}
	for _, adapter := range r.adapters {
		adapter.Shutdown()
	}
	close(r.output)
}

func (r *Router) printf(format string, args ...any) {
	r.println(fmt.Sprintf(format, args...))
}

func (r *Router) println(input ...any) {
	r.output <- message.Print{Obj: input}
}
