package main

import (
	"fmt"
	"slices"
	"time"
)

// Each cache entry has the following the data
type CacheEntry struct {
	offset uint32
	data string
	expiry int64
}

// The file name is instead used as a key for the cache map
// Go equivalent of Map<string, CacheEntry[]>
type CacheManager struct {
	cacheMap map[string][]CacheEntry
}

// Function to add a new entry to the cache map
func (cm *CacheManager) AddEntry(filename string, offset uint32, data string) {
	// Add the interval to the current time. This is when the cache entry expires
	time_now := time.Now()
	expiry := time_now.Add(time.Duration(t) * time.Millisecond).UnixMilli()
	fmt.Printf("%s Adding cache entry of length %d for %s starting at offset %d. Valid until %d (%d + %d)\n", header(CacheHeader), len(data), filename, offset, expiry, time_now.UnixMilli(), t)
	entry := CacheEntry{offset: offset, data: data, expiry: expiry}

	// Append to the CacheEntry array for this file
	cm.cacheMap[filename] = append(cm.cacheMap[filename], entry)
}

// Function to check if the cache has a specific entry
func (cm *CacheManager) GetEntry(filename string, offset uint32, amount uint32) (bool,string) {
	fmt.Printf("%s Checking if an entry of size %d for %s starting at offset %d exists\n", header(CacheHeader), amount, filename, offset)
	// Loop through all the entries belonging to the given file
	if entries, ok := cm.cacheMap[filename]; ok {
		// Reverse loop to make deletions simultaneously
		for i := len(entries)-1; i >=0; i-- {
			entry := entries[i]
			time_now := time.Now().UnixMilli()
			// Remove an entry if it has expired
			if entry.expiry < time_now {
				fmt.Printf("%s Deleting an old entry of size %d starting at offset %d. Entry was valid till %d but the time now is %d\n", header(CacheHeader), len(entry.data), entry.offset, entry.expiry, time_now)
				cm.cacheMap[filename] = slices.Delete(cm.cacheMap[filename], i, i+1)
			} else {
				// If the entry is valid then we need to check if requested range is a subset of the entry's range
				// offset must be [entry offset, entry offset+entry data length)
				// length must be [1, entry offset + entry data length - offset]
				if offset >= entry.offset && offset < entry.offset + uint32(len(entry.data)) && amount <= entry.offset + uint32(len(entry.data)) - offset {
					fmt.Printf("%s Found a valid entry set to expire at %d. Time now is %d\n", header(CacheHeader), entry.expiry, time_now)
					return true, entry.data[offset-entry.offset:offset-entry.offset+amount]
				}
			}
		}
	}
	fmt.Printf("%s Could not find any valid entry\n", header(CacheHeader))
	return false, ""
}