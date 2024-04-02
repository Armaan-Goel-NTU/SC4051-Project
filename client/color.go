package main

const (
    Reset   = "\033[0m"
    Red     = "\033[1;31m"
    Green   = "\033[1;32m"
    Yellow  = "\033[1;33m"
    Blue    = "\033[1;34m"
    Magenta = "\033[1;35m"
    Cyan    = "\033[1;36m"
    White   = "\033[1;37m"
)

func color(col string, text string) string {
    return col + text + Reset
}