all:
	@cargo vendor > config
	@./lic.py | sort  > licenses.txt
	@grep -E "checksum.*crates" Cargo.lock | cut -d ' ' -f2,3 | column -t > README.txt
test:
	@cargo vendor > config
	@grep -E "checksum.*crates" Cargo.lock | cut -d ' ' -f2,3 > README.txt
clean:
	@rm -rf vendor
	@rm -rf Cargo.lock
