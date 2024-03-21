package main

import (
	"bufio"
	"flag"
	"fmt"
	"net"
	"os"
	"strconv"
	"time"
)

var (
    s_port int
    c_port int
    host string
    t int
)

func checkError(err error) {
    if err != nil {
        fmt.Println(err)
        os.Exit(1)
    }
}

func readInt(prompt string) int {
	var choice int
	var err error = fmt.Errorf("")
	for err != nil {
		fmt.Print(prompt)
		reader := bufio.NewReader(os.Stdin)
		input, _ := reader.ReadString('\n')
		choice, err = strconv.Atoi(input[:len(input)-1])
	}
	return choice
}

func readString(prompt string) string {
	var input string
	for len(input) == 0 {
		fmt.Print(prompt)
		reader := bufio.NewReader(os.Stdin)
		input, _ = reader.ReadString('\n')
	}
	return input[:len(input)-1]
}

func displayMenu() int {
    fmt.Println("Menu:")
	fmt.Println("1. Read")
	fmt.Println("2. Insert")
	fmt.Println("3. Update")
	fmt.Println("4. Delete")
	fmt.Println("5. Monitor")
	fmt.Println("6. Exit")
	return readInt("Choose an option: ")
}

func main() {
    flag.StringVar(&host, "host", "127.0.0.1", "server host")
    flag.IntVar(&s_port, "s_port", 45600, "server port")
    flag.IntVar(&c_port, "c_port", 45601, "client port")
    flag.IntVar(&t, "t", 3, "freshness interval")
	flag.Parse();

    server_address := host + ":" + strconv.Itoa(s_port);
    client_address := "127.0.0.1:" + strconv.Itoa(c_port);

    s_udpAddr, err := net.ResolveUDPAddr("udp4", server_address)
    checkError(err);

    c_udpAddr, err := net.ResolveUDPAddr("udp4", client_address)
    checkError(err);
    
    c, err := net.DialUDP("udp4", c_udpAddr, s_udpAddr)
    checkError(err);

    for {
		choice := displayMenu()
		switch choice {
		case 1:
			req := Request(Read)
			req.AddStringInput("File Path: ")
			req.AddIntInput("Offset: ")
			req.AddIntInput("Amount: ")
			Send(c, req)
		case 2:
			req := Request(Insert)
			req.AddStringInput("File Path: ")
			req.AddIntInput("Offset: ")
			req.AddStringInput("Data: ")
			Send(c, req)
		case 3:
			req := Request(Update)
			req.AddStringInput("File Path: ")
			req.AddIntInput("Offset: ")
			req.AddStringInput("Data: ")
			Send(c, req)
		case 4:
			req := Request(Delete)
			req.AddStringInput("File Path: ")
			req.AddIntInput("Offset: ")
			req.AddIntInput("Amount: ")
			Send(c, req)
		case 5:
			req := Request(Monitor)
			req.AddStringInput("File Path: ")
			interval := readInt("Interval (ms): ")
			req.AddInt(interval)
			Send(c, req)
			for {
				buf := make([]byte, 1024)
				c.SetReadDeadline(time.Now().Add(time.Duration(interval) * time.Millisecond))
				amt, err := c.Read(buf)
				if err != nil {
					if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
						fmt.Println("Monitor interval has passed!")
						break
					}
					fmt.Println("Error reading:", err)
					continue
				} else {
					fmt.Printf("Read %d bytes\n", amt)
					response := Response(buf)
					if response.status == Bad {
						fmt.Println("Error")
					} else {
						fmt.Println("File Changed!")
					}
					fmt.Println(response.data)
				}
			}
		case 6:
            fmt.Println("Exiting…")
            c.Close();
			os.Exit(0)
		default:
			fmt.Println("Invalid choice. Please choose a number between 1 and 5.")
		}
	}
}