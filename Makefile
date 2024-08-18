PREFIX = $(DESTDIR)/usr/local
BINDIR = $(PREFIX)/bin
MANDIR = $(PREFIX)/share/man

all: pesticide

pesticide: *.go
	go build -o pesticide

install:
	install -d \
		$(BINDIR) \
		$(MANDIR)/man1/
	install -pm 0755 pesticide $(BINDIR)/

uninstall:
	rm -f \
		$(BINDIR)/pesticide

clean:
	go clean
	rm -f pesticide

.PHONY: install uninstall clean
