package main

type Operation int
const (
	Read Operation = 0
	Insert Operation = 1
	Update Operation = 2
	Delete Operation = 3
	Monitor Operation = 4
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
	c.buf = append(c.buf, uint8(op))
	return c
}

func (c *ClientMarshal) AddIntInput(prompt string) {
	val := readInt(prompt)
	c.AddInt(val)
}

func (c *ClientMarshal) AddInt(val int) {
	c.buf = append(c.buf, uint8((val >> 24) & 0xFF), uint8((val >> 16) & 0xFF), uint8((val >> 8) & 0xFF), uint8(val & 0xFF))
}

func (c *ClientMarshal) AddStringInput(prompt string) {
	val := readString(prompt)
	c.AddInt(len(val))
	for _, char := range val {
        c.buf = append(c.buf, uint8(char))
    }
}

type ServerUnmarshal struct {
	status Status
	data string
}

func Response(raw []byte) ServerUnmarshal {
	return ServerUnmarshal{status: Status(raw[0]), data: string(raw[1:])}
}