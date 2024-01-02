.PHONY: clean

into-string:
	ghc -Wall -XSafe -fforce-recomp --make into_string.hs -o into-string && ./into-string

clean:
	rm *.hi *.o into-string
