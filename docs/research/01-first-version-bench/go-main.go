package main

import (
	"bufio"
	"context"
	"encoding/json"
	"log"
	"net"
	"os"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

type In struct{}
type Out struct {
	Raw json.RawMessage `json:"raw"`
}

func ping(ctx context.Context, req *mcp.CallToolRequest, _ In) (*mcp.CallToolResult, Out, error) {
	c, err := net.Dial("unix", os.Getenv("HERDR_SOCKET_PATH"))
	if err != nil {
		return nil, Out{}, err
	}
	defer c.Close()
	c.Write([]byte(`{"id":"1","method":"ping","params":{}}` + "\n"))
	line, err := bufio.NewReader(c).ReadBytes('\n')
	if err != nil {
		return nil, Out{}, err
	}
	return nil, Out{Raw: line[:len(line)-1]}, nil
}

func main() {
	s := mcp.NewServer(&mcp.Implementation{Name: "bench", Version: "0"}, nil)
	mcp.AddTool(s, &mcp.Tool{Name: "ping", Description: "ping herdr"}, ping)
	if err := s.Run(context.Background(), &mcp.StdioTransport{}); err != nil {
		log.Fatal(err)
	}
}
