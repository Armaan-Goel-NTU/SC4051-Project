package main

import (
	"fmt"
	"net"
	"os"
	"strconv"
	"time"
)

// the globally accessible connection handle is defined here
var (
	conn *net.UDPConn
)

// pretty-prints an error
func printError(err string) {
	fmt.Printf("%s %s\n", header(Error), err)
}

/* Checks if an error exists, prints it and exits application with error code 1.
   Used for the initial UDP port binding where an error means 
   we cannot even send a request to the server */
func checkError(err error) {
    if err != nil {
		printError(err.Error())
        os.Exit(1)
    }
}

/* Close the connection and exits without an error 
   Used when disconnecting manually or if handshake fails */
func exit() {
	fmt.Printf("%s Closing connection and exiting\n", header(ClientHeader))
	conn.Close()
	os.Exit(0)
}

// Function used to initially connect to the server
func ConnectToServer() {
	// define addresses. client is also bound to a specific port
	server_address := s_host + ":" + strconv.Itoa(s_port);
    client_address := c_host + ":" + strconv.Itoa(c_port);

	// resolve the constructed addresses
    s_udpAddr, err := net.ResolveUDPAddr("udp4", server_address)
    checkError(err)

    c_udpAddr, err := net.ResolveUDPAddr("udp4", client_address)
    checkError(err)
    
	// DialUDP creates a UDP connection and provides a Write method to directly write to the broadcast
    conn, err = net.DialUDP("udp4", c_udpAddr, s_udpAddr)
    checkError(err)
	
	// the session_id is supposed to be unique to a client, so the current time works.
	// epoch time in seconds being stored in an unsigned integer will work till 2106, which should be good enough
	session_id := uint32(time.Now().Unix())
    fmt.Printf("\n%s Attempting to handshake with the server with session id %d\n", header(ClientHeader), session_id)
	
	// Initialize and send a request to the server
	handshake := Request(Handshake)
	handshake.AddInt(session_id)
	success, _ := Send(handshake)

	// exit if unsuccessful
	if !success { exit() }
}

// Function used to safely disconnect to the server
func DisconnectFromServer() {
	// Initialize and send a request to the server
	fmt.Printf("\n%s Sending disconnect notice\n", header(ClientHeader))
	req := Request(Disconnect)
	Send(req)

	// exit regardless of success
	exit()
}

// Function used to wait for monitor updates and process them
func WaitForUpdates(wait_until time.Time, path string) {
	fmt.Printf("%s Entering Monitor Mode\n", header(MonitorHeader))
	for { // while(True)
		buf := make([]byte, 1024 * 1024)

		/* wait_until is the monitor expiry time
		   setReadDeadline will throw a timeout error if it's waiting for
		   some data and the expiry time passes */
		conn.SetReadDeadline(wait_until) 
		
		amt, err := conn.Read(buf)
		if err != nil {
			// timeout error indicates monitor interval has passed which means we need to exit monitor mode
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				fmt.Printf("%s Monitor interval has passed! Interval ended at %d. Time now is %d\n", header(MonitorHeader), wait_until.UnixMilli(), time.Now().UnixMilli())
				break
			}
			// pretty print any other error and try again
			printError(err.Error())
			continue
		} else {
			// process data that is received
			fmt.Printf("%s Received %d bytes\n", header(UDPHeader), amt)

			// wrap received data to a Response and print it out
			response := Response(buf, amt)
			col := header(ServerGood)
			if response.status == Bad {
				col = header(ServerBad)
			} else {
				fmt.Printf("%s File Changed!\n",  header(MonitorHeader))
				/* we add this data to the cache
				   but we only do this when the time to expiry is less than the freshness interval
				   otherwise it's a waste */
				if time.Until(wait_until).Milliseconds() < int64(t) {
					cache_manager.AddEntry(path, 0, response.data)
				}
			}
			fmt.Printf("%s %s\n", col, response.data)
		}
	}
}

/* function to manage retry attempts when sending request to the server
	it is used for both a write and a read error */
func CheckAttempts(attempts int) int {
	if attempts > 0 {
		attempts--
		fmt.Printf("%s Retrying! %d attempts left\n", header(ClientHeader) ,attempts)
	} else {
		printError("Operation Failed. Reached max retries (" + strconv.Itoa(retries) + ")")
	}
	return attempts
}

// All requests are sent via this function
func Send(req *RequestMarshal) (bool, string) {
	attempts := retries
	for attempts > 0 { 
		_, err := conn.Write(req.buf)
		fmt.Printf("%s Sent %d bytes\n", header(UDPHeader), len(req.buf))
		
		// handle write error by printing it and retrying
		if err != nil {
			printError(err.Error())
			attempts = CheckAttempts(attempts)
			if attempts == 0 { break }
			continue
		}

		// wait for a response till timeout if no write error
		p :=  make([]byte, 1024 * 1024)
		conn.SetReadDeadline(time.Now().Add(time.Duration(timeout) * time.Millisecond))
		amt, err := conn.Read(p)
		if err != nil {
			// handle timeout errors separately as this could indicate message loss
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				printError("Response Timed Out")
			} else {
				printError(err.Error())
			}
			// check if we need to retry
			attempts = CheckAttempts(attempts)
			if attempts == 0 { break }
		} else {
			// wrap received data to a Response and print it out
			fmt.Printf("%s Received %d bytes\n", header(UDPHeader), amt)
			response := Response(p, amt)
			col := header(ServerGood)
			if response.status == Bad {
				col = header(ServerBad)
			}
			fmt.Printf("%s %s\n", col, response.data)
			return response.status == Good, response.data
		}
	}
	return false, ""
}