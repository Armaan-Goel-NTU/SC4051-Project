package main

import (
	"fmt"
	"net"
)

func awaitResponse(c *net.UDPConn) {
	p :=  make([]byte, 1024 * 1024)
	amt, err := c.Read(p)
	if err != nil {
		fmt.Println(err)
	} else {
		fmt.Printf("Read %d bytes\n", amt)
		response := Response(p)
		if response.status == Bad {
			fmt.Println("Error")
		} else {
			fmt.Println("Success")
		}
		fmt.Println(response.data)
	}
}

func Send(c *net.UDPConn, req *ClientMarshal) {
	_, err := c.Write(req.buf)
	checkError(err)
	awaitResponse(c);
}