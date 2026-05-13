# Generate a heap profile and open it interactively
& "C:\Program Files\Go\bin\go.exe" test -v -run TestConvertExcelToPDFWithExcel_MemoryProfile -memprofile mem.pprof ..
#& "C:\Program Files\Go\bin\go.exe" tool pprof mem.pprof

# Run the benchmark with alloc reporting
#& "C:\Program Files\Go\bin\go.exe" test -bench BenchmarkConvertExcelToPDFWithExcel_Concurrent -benchmem -count 3 ..