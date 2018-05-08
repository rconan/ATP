NOWEBPATH	= /usr
WEAVE   	= $(NOWEBPATH)/bin/noweave
TANGLE    	= $(NOWEBPATH)/bin/notangle

all: yaml python doc

yaml: atp.nwy atp.yaml

python: atp.nwy atp.py
	$(TANGLE) -Rmain.py atp.nwy > main.py

doc: atp.nwy atp.tex
	make -C doc/ all

.SUFFIXES: .nwy .tex .py .yaml

.nwy.tex:
	$(WEAVE) -delay -index $< > doc/$@

.nwy.py:
	$(TANGLE) -R$@ $< > $@

.nwy.yaml:
	$(TANGLE) -R$@ $< > $@
 	
