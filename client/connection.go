package main

import (
	"fmt"
	"net"
	"os"
	"strconv"
	"time"
)

var (
	conn *net.UDPConn
)

func printError(err string) {
	fmt.Printf("%s %s\n", header(Error), err)
}

func checkError(err error) {
    if err != nil {
		printError(err.Error())
        os.Exit(1)
    }
}

func exit() {
	fmt.Printf("%s Closing connection and exiting\n", header(ClientHeader))
	conn.Close()
	os.Exit(0)
}

func ConnectToServer() {
	server_address := host + ":" + strconv.Itoa(s_port);
    client_address := c_addr + ":" + strconv.Itoa(c_port);

    s_udpAddr, err := net.ResolveUDPAddr("udp4", server_address)
    checkError(err)

    c_udpAddr, err := net.ResolveUDPAddr("udp4", client_address)
    checkError(err)
    
    conn, err = net.DialUDP("udp4", c_udpAddr, s_udpAddr)
    checkError(err)
	
	session_id := uint32(time.Now().Unix())
    fmt.Printf("\n%s Attempting to handshake with the server with session id %d\n", header(ClientHeader), session_id)
	handshake := Request(Handshake)
	handshake.AddInt(session_id)
	success, _ := Send(handshake)
	if !success { exit() }
}

func DisconnectFromServer() {
	fmt.Printf("\n%s Sending disconnect notice\n", header(ClientHeader))
	req := Request(Disconnect)
	Send(req)
	exit()
}

func WaitForUpdates(wait_until time.Time, path string) {
	fmt.Printf("%s Entering Monitor Mode\n", header(MonitorHeader))
	for {
		buf := make([]byte, 1024)
		conn.SetReadDeadline(wait_until)
		
		amt, err := conn.Read(buf)
		if err != nil {
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				fmt.Printf("%s Monitor interval has passed! Interval ended at %d. Time now is %d\n", header(MonitorHeader), wait_until.UnixMilli(), time.Now().UnixMilli())
				break
			}
			printError(err.Error())
			continue
		} else {
			fmt.Printf("%s Received %d bytes\n", header(UDPHeader), amt)
			response := Response(buf, amt)
			col := header(ServerGood)
			if response.status == Bad {
				col = header(ServerBad)
			} else {
				fmt.Printf("%s File Changed!\n",  header(MonitorHeader))
				if time.Until(wait_until).Milliseconds() < int64(t) {
					cache_manager.AddEntry(path, 0, response.data)
				}
			}
			fmt.Printf("%s %s\n", col, response.data)
		}
	}
}

func CheckAttempts(attempts int) int {
	if attempts > 0 {
		attempts--
		fmt.Printf("%s Retrying! %d attempts left\n", header(ClientHeader) ,attempts)
	} else {
		printError("Operation Failed. Reached max retries (" + strconv.Itoa(retries) + ")")
	}
	return attempts
}

func Send(req *RequestMarshal) (bool, string) {
	attempts := retries
	for attempts > 0 {
		_, err := conn.Write(req.buf)
		fmt.Printf("%s Sent %d bytes\n", header(UDPHeader), len(req.buf))
		if err != nil {
			printError(err.Error())
			attempts = CheckAttempts(attempts)
			if attempts == 0 { break }
			continue
		}
		p :=  make([]byte, 1024 * 1024)
		conn.SetReadDeadline(time.Now().Add(time.Duration(timeout) * time.Millisecond))
		amt, err := conn.Read(p)
		if err != nil {
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				printError("Response Timed Out")
			} else {
				printError(err.Error())
			}
			attempts = CheckAttempts(attempts)
			if attempts == 0 { break }
		} else {
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