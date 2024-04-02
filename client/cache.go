package main

import (
	"fmt"
	"slices"
	"time"
)

type CacheEntry struct {
	offset uint32
	data string
	expiry int64
}

type CacheManager struct {
	cacheMap map[string][]CacheEntry
}

func (cm *CacheManager) AddEntry(filename string, offset uint32, data string) {
	time_now := time.Now()
	expiry := time_now.Add(time.Duration(t) * time.Millisecond).UnixMilli()
	fmt.Printf("%s Adding cache entry of length %d for %s starting at offset %d. Valid until %d (%d + %d)\n", color(Magenta, "[CacheManager]:"), len(data), filename, offset, expiry, time_now.UnixMilli(), t)
	entry := CacheEntry{offset: offset, data: data, expiry: expiry}
	cm.cacheMap[filename] = append(cm.cacheMap[filename], entry)
}

func (cm *CacheManager) GetEntry(filename string, offset uint32, amount uint32) (bool,string) {
	fmt.Printf("%s Checking if an entry of size %d for %s starting at offset %d exists\n", color(Magenta, "[CacheManager]:"), amount, filename, offset)
	if entries, ok := cm.cacheMap[filename]; ok {
		for i := len(entries)-1; i >=0; i-- {
			entry := entries[i]
			time_now := time.Now().UnixMilli()
			if entry.expiry < time_now {
				fmt.Printf("%s Deleting an old entry of size %d starting at offset %d. Entry was valid till %d but the time now is %d\n", color(Magenta, "[CacheManager]:"), len(entry.data), entry.offset, entry.expiry, time_now)
				cm.cacheMap[filename] = slices.Delete(cm.cacheMap[filename], i, i+1)
			} else {
				if offset >= entry.offset && offset < entry.offset + uint32(len(entry.data)) && amount <= entry.offset + uint32(len(entry.data)) - offset {
					fmt.Printf("%s Found a valid entry set to expire at %d. Time now is %d\n", color(Magenta, "[CacheManager]:"), entry.expiry, time_now)
					return true, entry.data[offset-entry.offset:offset-entry.offset+amount]
				}
			}
		}
	}
	fmt.Printf("%s Could not find any valid entry\n", color(Magenta, "[CacheManager]:"))
	return false, ""
}