NOWEBPATH	= /usr
WEAVE   	= $(NOWEBPATH)/bin/noweave
TANGLE    	= $(NOWEBPATH)/bin/notangle

all: yaml python doc

yaml: atp.nw atp.yml

python: atp.nw atp.py

doc: atp.nw atp.tex
	make -C doc/ all

.SUFFIXES: .nw .tex .py .yaml

.nw.tex:
	$(WEAVE) -delay -index $< > doc/$@

.nw.py:
	$(TANGLE) -R$@ $< > $@

.nw.yaml:
	$(TANGLE) -R$@ $< > $@
