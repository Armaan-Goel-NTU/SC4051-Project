package main

import "fmt"

// used to keep track of the request number internally
// this will be sent to the server as well
// each session starts with a request number of 0
var (
	reqNo uint32 = 0
)

// Operation constants for easy access and modification if need be
type Operation int
const (
	Handshake Operation = 0
	Disconnect Operation = 1
	Read Operation = 2
	Insert Operation = 3
	Update Operation = 4
	Delete Operation = 5
	Monitor Operation = 6
)

// A container for the request buffer to build functions on top of it
type RequestMarshal struct {
	buf []uint8
}

/* Creates a new request container.
   Always adds the operation as a the first byte and then a 4-byte request number */
func Request(op Operation) *RequestMarshal {
	c := &RequestMarshal{buf: make([]uint8, 0)}
	c.AddInt(reqNo)
	c.buf = append(c.buf, uint8(op))
	// increment the request number for the next request
	reqNo++
	return c
}

// Marshalls an integer into 4 bytes
// the (c *RequestMarshal) before the function name assigns this function to the RequestMarshal struct
func (c *RequestMarshal) AddInt(val uint32) {
	c.buf = append(c.buf, uint8((val >> 24) & 0xFF), uint8((val >> 16) & 0xFF), uint8((val >> 8) & 0xFF), uint8(val & 0xFF))
}

// Marshalls a string into 4 + n bytes
func (c *RequestMarshal) AddString(val string) {
	// Add the length as an unsigned integer first
	c.AddInt(uint32(len(val)))

	// Add all the bytes of the string
	for _, char := range val {
        c.buf = append(c.buf, uint8(char))
    }
}

/* A function to print the request number. 
   It needs to be called after the Request is actually made for neater output */
func (c *RequestMarshal) PrintNumber() {
	fmt.Printf("%s Creating req no. %d\n", header(ClientHeader), reqNo - 1)
}

// Reads an integer input from console and adds it to the request buffer
func (c *RequestMarshal) AddIntInput(prompt string, min uint32) uint32 {
	val := readInt(prompt, min)
	c.AddInt(val)
	return val
}

// Reads a line from console and adds it to the request buffer
func (c *RequestMarshal) AddStringInput(prompt string) string {
	val := readString(prompt)
	c.AddString(val)
	return val
}

// Status type as received from the server
type Status int
const (
	Bad Status = 0
	Good Status = 1
)

// A container for the response given by a server
type ResponseHandler struct {
	status Status
	data string
}

/* Divides the data received from the server into a 1-byte status
   and an n-1 byte string wrapping it into the ResponseHandler container */
func Response(raw []byte, amt int) ResponseHandler {
	return ResponseHandler{status: Status(raw[0]), data: string(raw[1:amt])}
}