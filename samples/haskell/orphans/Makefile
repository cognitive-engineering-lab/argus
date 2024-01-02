.PHONY: clean

orphan:
	ghc -Wall -XSafe -fforce-recomp --make D.hs -o orphan && ./orphan

clean:
	rm *.hi *.o orphan
