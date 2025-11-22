# SPDX-License-Identifier: GPL-2.0

KDIR ?= ../linux-6.17.8

default:
	$(MAKE) -C $(KDIR) M=$$PWD

modules_install: default
	$(MAKE) -C $(KDIR) M=$$PWD modules_install

clean:
	$(MAKE) -C $(KDIR) M=$$PWD clean
