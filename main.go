package main

import (
	"bufio"
	"os"

	"github.com/google/go-dap"
)

func main() {
	reader := bufio.NewReader(os.Stdin)

	for {
		msg, err := dap.ReadProtocolMessage(reader)
		if err != nil {
			panic(err)
		}
	}
}
