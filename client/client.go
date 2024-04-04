package main

import (
	"bufio"
	"flag"
	"fmt"
	"os"
	"strconv"
)

// global variables (command line arguments)
var (
    s_port int
    c_port int
    host string
    t int
	c_addr string = "127.0.0.1"
	retries int
	timeout int
)

/* This function keeps reading a line from the console input
   until the length is greater than 0 */
   func readString(prompt string) string {
	var input string
	for len(input) == 0 { // this is a while loop in Go
		fmt.Print(prompt)
		reader := bufio.NewReader(os.Stdin)
		input, _ = reader.ReadString('\n')
	}
	// -1 to remove the newline
	return input[:len(input)-1]
}

/* This function keeps reading a string from the console input
   until it can be correctly converted to an integer and is greater than the minimum */
func readInt(prompt string, min uint32) uint32 {
	var choice uint32
	var choice64 uint64
	var err error = fmt.Errorf("")
	for err != nil { 
		input := readString(prompt)
		// -1 to remove the newline. 10 is the base. 32 is the bit-width of the integer
		choice64, err = strconv.ParseUint(input, 10, 32) 
		choice = uint32(choice64)
		if choice < min {
			err = fmt.Errorf("%d is less than the minimum %d",choice,min)
		}
	}
	return choice
}

// This function displays the menu and asks for user choice 
func displayMenu() uint32 {
	fmt.Println()
    fmt.Println(header(Menu))
	fmt.Println("1. Read")
	fmt.Println("2. Insert")
	fmt.Println("3. Update")
	fmt.Println("4. Delete")
	fmt.Println("5. Monitor")
	fmt.Println("6. Exit")
	return readInt("Choose an option: ", 0)
}

func main() {
	/* Command Line Arguments in the format 
		flag.Type(&variable, argument name, default value, description) */
    flag.StringVar(&host, "host", "127.0.0.1", "server host")
    flag.IntVar(&s_port, "s_port", 45600, "server port")
    flag.IntVar(&c_port, "c_port", 45601, "client port")
    flag.IntVar(&t, "t", 10000, "freshness interval")
	flag.IntVar(&retries, "retries", 3, "number of request retries")
	flag.IntVar(&timeout, "timeout", 3000, "response timeout")
	flag.Parse()

	// Print all the command line arguments for verification
	fmt.Printf("%s Server address is %s:%d\n", header(ClientHeader), host, s_port)
	fmt.Printf("%s Client address is %s:%d\n", header(ClientHeader), c_addr, c_port)
	fmt.Printf("%s Freshness interval is %dms\n", header(ClientHeader), t)
	fmt.Printf("%s Max retries is %d\n", header(ClientHeader), retries)
	fmt.Printf("%s Timeout is %dms\n", header(ClientHeader), timeout)

	// Connect to the server and create a cache manager for this run
	ConnectToServer()
	cache_manager = CacheManager{cacheMap: make(map[string][]CacheEntry)}

	// Continously runs displayMenu() and calls the appropriate callback function
	// Program exits with option 6 (DisconnectFromServer())
    for {
		choice := displayMenu()
		switch choice {
			case 1:
				ReadService()
			case 2:
				InsertService()
			case 3:
				UpdateService()
			case 4:
				DeleteService()
			case 5:
				MonitorService()
			case 6:
				DisconnectFromServer()
			default:
				fmt.Println("Invalid choice. Please choose a number between 1 and 6.")
		}
	}
}
