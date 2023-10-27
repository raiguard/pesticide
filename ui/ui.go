package ui

import (
	"fmt"

	"github.com/raiguard/pesticide/command"
	"github.com/wader/readline"
)

func Run() {
	inputOut := make(chan string)

	go inputWorker(inputOut)
	for line := range inputOut {
		cmd, err := command.Parse(line)
		if err != nil {
			fmt.Println("error:", err)
			continue
		}
		fmt.Printf("%+v\n", cmd)
		if cmd.Noun == "quit" {
			close(inputOut)
			break
		}
	}
}

func inputWorker(out chan string) {
	for {
		rl, err := readline.New("(pesticide) ")
		if err != nil {
			panic(err)
		}
		for {
			line, err := rl.Readline()
			if err != nil { // io.EOF
				break
			}
			out <- line
		}
	}
}
