package main

import (
	"fmt"
	"time"
)

var (
	cache_manager CacheManager
)

func ReadService() {
	path := readString("File Path: ")
	offset := readInt("Offset: ")
	amount := readInt("Amount: ")
	fmt.Println()

	has_entry, entry := cache_manager.GetEntry(path, offset, amount)
	if has_entry {
		fmt.Printf("%s %s\n", header(ClientHeader), entry)
	} else {
		req := Request(Read)
		req.AddString(path)
		req.AddInt(offset)
		req.AddInt(amount)
		
		req.PrintNumber()
		fmt.Printf("%s Sending request to read %d bytes starting at offset %d in %s\n", header(ClientHeader), amount, offset, path)
		success, data := Send(req)
		if success {
			cache_manager.AddEntry(path, offset, data)
		}
	}
}

func InsertService() {
	req := Request(Insert)
	path := req.AddStringInput("File Path: ")
	offset := req.AddIntInput("Offset: ")
	data := req.AddStringInput("Data: ")
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to insert given data of length %d at offset %d in %s\n", header(ClientHeader), len(data), offset, path)
	Send(req)
}

func UpdateService() {
	req := Request(Update)
	path := req.AddStringInput("File Path: ")
	offset := req.AddIntInput("Offset: ")
	data := req.AddStringInput("Data: ")
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to update given data of length %d at offset %d in %s\n", header(ClientHeader), len(data), offset, path)
	Send(req)
}

func DeleteService() {
	req := Request(Delete)
	path := req.AddStringInput("File Path: ")
	offset := req.AddIntInput("Offset: ")
	amount := req.AddIntInput("Amount: ")
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to delete %d bytes starting at offset %d in %s\n", header(ClientHeader), amount, offset, path)
	Send(req)
}

func MonitorService() {
	req := Request(Monitor)
	path := req.AddStringInput("File Path: ")
	interval := req.AddIntInput("Interval (ms): ")
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to monitor any updates to %s for the next %dms\n", header(ClientHeader), path, interval)
	success, _ := Send(req)
	
	if success {
		monitor_end := time.Now().Add(time.Duration(interval) * time.Millisecond)
		WaitForUpdates(monitor_end, path)
	}
}
