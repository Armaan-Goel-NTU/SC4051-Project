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

const (
    ClientHeader = iota
    MonitorHeader
    ServerGood
    ServerBad
    CacheHeader 
    Menu
    UDPHeader
    Error  
)

func header(i int) string {
    switch i {
        case ClientHeader:
            return Yellow + "[Client]:" + Reset
        case MonitorHeader:
            return Cyan + "[Monitor]:" + Reset
        case ServerGood:
            return Green + "[Server]:" + Reset
        case ServerBad:
            return Red + "[Server]:" + Reset
        case CacheHeader:
            return Magenta + "[CacheManager]:" + Reset
        case Menu:
            return White + "Menu" + Reset
        case UDPHeader:
            return Blue + "[UDP]:" + Reset
        case Error:
            return Red + "[Error]:" + Reset
        default:
            return White + "[Unknown]:" + Reset
    }
}
