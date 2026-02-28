def stress-test-fim [
  total_files: int = 5000,
  percent_modify: int = 5
] {
let dir = "fim_stress_test"
  if ($dir | path exists) {
    print $"[*] Removing old test directory..."
    rm -r -f $dir
  }
  mkdir $dir
  print $"[*] Generating ($total_files) files..."
  1..$total_files | par-each { |i|
    $"Baseline data for file ($i)\n" | save $"($dir)/file_($i).txt"
  }
  print "[*] File generation complete."
  print "Open another terminal and run:"
  print $"   runhaskell Fim.hs init ($dir)"
  print "Press Enter here when the baseline snapshot is done..."
  print ""
  input
  let modify_count = (($total_files * $percent_modify) / 100) | into int
  print $"[*] Tampering with the first ($modify_count) files..."
  1..$modify_count | par-each { |i|
    $"Tampered data!\n" | save --append $"($dir)/file_($i).txt"
  }
  let delete_count = 10
  print $"[*] Deleting ($delete_count) files..."
  (($total_files - $delete_count + 1)..$total_files) | each { |i|
    rm $"($dir)/file_($i).txt"
  }
  print "[*] Dropping 3 rogue files into the directory..."
  "Malware payload A" | save $"($dir)/rogue_A.bin"
  "Malware payload B" | save $"($dir)/rogue_B.bin"
  "Malware payload C" | save $"($dir)/rogue_C.bin"
  print ""
  print "[+] Stress test environment is ready"
  print "Run your FIM check command now:"
  print $"    runhaskell Fim.hs check ($dir)"
}
