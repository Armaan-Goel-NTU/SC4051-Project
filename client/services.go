package main

import (
	"fmt"
	"time"
)

// the globally accessible CacheManager is defined here
var (
	cache_manager CacheManager
)

// Function for the read service
func ReadService() {
	// Read all the parameters from the console first
	path := readString("File Path: ")
	offset := readInt("Offset: ", 0)
	amount := readInt("Amount: ", 1)
	fmt.Println()

	// Check if the cache can service this request 
	has_entry, entry := cache_manager.GetEntry(path, offset, amount)
	if has_entry {
		// Return the data from cache
		fmt.Printf("%s %s\n", header(ClientHeader), entry)
	} else {
		// Initialize and send a request to the server
		req := Request(Read)
		req.AddString(path)
		req.AddInt(offset)
		req.AddInt(amount)
		
		req.PrintNumber()
		fmt.Printf("%s Sending request to read %d bytes starting at offset %d in %s\n", header(ClientHeader), amount, offset, path)
		success, data := Send(req)
		if success {
			// save the data received as a new entry in the cache if it was successful
			cache_manager.AddEntry(path, offset, data)
		}
	}
}

// Function for the insert service
func InsertService() {
	// Initialize and send a request to the server
	req := Request(Insert)
	path := req.AddStringInput("File Path: ")
	offset := req.AddIntInput("Offset: ", 0)
	data := req.AddStringInput("Data: ")
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to insert given data of length %d at offset %d in %s\n", header(ClientHeader), len(data), offset, path)
	Send(req)
}

// Function for the update service
func UpdateService() {
	// Initialize and send a request to the server
	req := Request(Update)
	path := req.AddStringInput("File Path: ")
	offset := req.AddIntInput("Offset: ", 0)
	data := req.AddStringInput("Data: ")
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to update given data of length %d at offset %d in %s\n", header(ClientHeader), len(data), offset, path)
	Send(req)
}

// Function for the delete service
func DeleteService() {
	// Initialize and send a request to the server
	req := Request(Delete)
	path := req.AddStringInput("File Path: ")
	offset := req.AddIntInput("Offset: ", 0)
	amount := req.AddIntInput("Amount: ", 1)
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to delete %d bytes starting at offset %d in %s\n", header(ClientHeader), amount, offset, path)
	Send(req)
}

// Function for the monitor service
func MonitorService() {
	// Initialize and send a request to the server
	req := Request(Monitor)
	path := req.AddStringInput("File Path: ")
	interval := req.AddIntInput("Interval (ms): ", 1)
	
	fmt.Println()
	req.PrintNumber()
	fmt.Printf("%s Sending request to monitor any updates to %s for the next %dms\n", header(ClientHeader), path, interval)
	success, _ := Send(req)
	
	if success {
		// start monitoring changes until the interval is passed if it was successful
		monitor_end := time.Now().Add(time.Duration(interval) * time.Millisecond)
		WaitForUpdates(monitor_end, path)
	}
}
