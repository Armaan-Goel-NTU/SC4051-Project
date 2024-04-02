package main

import (
	"bufio"
	"flag"
	"fmt"
	"os"
	"strconv"
)

var (
    s_port int
    c_port int
    host string
    t int
	c_addr string = "127.0.0.1"
	retries int
	timeout int
)

func readInt(prompt string) uint32 {
	var choice uint32
	var choice64 uint64
	var err error = fmt.Errorf("")
	for err != nil {
		fmt.Print(prompt)
		reader := bufio.NewReader(os.Stdin)
		input, _ := reader.ReadString('\n')
		choice64, err = strconv.ParseUint(input[:len(input)-1], 10, 32)
		choice = uint32(choice64)
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

func displayMenu() uint32 {
	fmt.Println()
    fmt.Println(color(White,"Menu:"))
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
    flag.IntVar(&t, "t", 10000, "freshness interval")
	flag.IntVar(&retries, "retries", 3, "number of request retries")
	flag.IntVar(&timeout, "timeout", 3000, "response timeout")
	flag.Parse()

	fmt.Printf("%s Server address is %s:%d\n", color(Yellow,"[Client]:"), host, s_port)
	fmt.Printf("%s Client address is %s:%d\n", color(Yellow,"[Client]:"), c_addr, c_port)
	fmt.Printf("%s Freshness interval is %dms\n", color(Yellow,"[Client]:"), t)
	fmt.Printf("%s Max retries is %d\n", color(Yellow,"[Client]:"), retries)
	fmt.Printf("%s Timeout is %dms\n", color(Yellow,"[Client]:"), timeout)

	ConnectToServer()
	cache_manager = CacheManager{cacheMap: make(map[string][]CacheEntry)}

    for {
		choice := displayMenu()
		switch choice {
			case 1:
				ReadBytes()
			case 2:
				InsertBytes()
			case 3:
				UpdateBytes()
			case 4:
				DeleteBytes()
			case 5:
				MonitorChanges()
			case 6:
				DisconnectFromServer()
			default:
				fmt.Println("Invalid choice. Please choose a number between 1 and 6.")
		}
	}
}