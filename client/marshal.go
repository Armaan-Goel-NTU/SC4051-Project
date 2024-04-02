package main

import "fmt"

var (
	reqNo uint32 = 0
)

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

type Status int
const (
	Bad Status = 0
	Good Status = 1
)

type ClientMarshal struct {
	buf []uint8
}

func Request(op Operation) *ClientMarshal {
	c := &ClientMarshal{buf: make([]uint8, 0)}
	c.AddInt(reqNo)
	c.buf = append(c.buf, uint8(op))
	reqNo++
	return c
}

func (c *ClientMarshal) AddIntInput(prompt string) uint32 {
	val := readInt(prompt)
	c.AddInt(val)
	return val
}

func (c *ClientMarshal) AddInt(val uint32) {
	c.buf = append(c.buf, uint8((val >> 24) & 0xFF), uint8((val >> 16) & 0xFF), uint8((val >> 8) & 0xFF), uint8(val & 0xFF))
}

func (c *ClientMarshal) AddString(val string) {
	c.AddInt(uint32(len(val)))
	for _, char := range val {
        c.buf = append(c.buf, uint8(char))
    }
}

func (c *ClientMarshal) PrintNumber() {
	fmt.Printf("%s Creating req no. %d\n", color(Yellow, "[Client]:"), reqNo - 1)
}

func (c *ClientMarshal) AddStringInput(prompt string) string {
	val := readString(prompt)
	c.AddString(val)
	return val
}

type ServerUnmarshal struct {
	status Status
	data string
}

func Response(raw []byte, amt int) ServerUnmarshal {
	return ServerUnmarshal{status: Status(raw[0]), data: string(raw[1:amt])}
}