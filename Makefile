# make run CPUS=<NUM>
QEMUFLAGS += -smp $(if $(CPUS),$(CPUS),1)

# make run GDBSERVER=1
ifneq ($(GDBSERVER),)
QEMUFLAGS += -S -gdb tcp::7777
endif

# make run QEMU_DEBUG=1
ifneq ($(QEMU_DEBUG),)
QEMUFLAGS += -d unimp,guest_errors,int,cpu_reset -D qemu-debug.log
endif

ifeq ($(V),)
.SILENT:
endif

GDB       ?= rust-gdb

# make build RELEASE=1
ifeq ($(RELEASE),)
BUILD_DIR := target/riscv64gc-unknown-none-elf/debug
else
BUILD_DIR := target/riscv64gc-unknown-none-elf/release
CARGO_FLAGS += --release
endif

QEMU ?= $(QEMU_PREFIX)qemu-system-riscv64
QEMUFLAGS += -machine virt -bios default -nographic -serial mon:stdio --no-reboot
QEMUFLAGS += -drive id=drive0,file=lorem.txt,format=raw,if=none
QEMUFLAGS += -device virtio-blk-device,drive=drive0,bus=virtio-mmio-bus.0

kernel_elf    := $(BUILD_DIR)/kernel
rootserver_elf      := $(BUILD_DIR)/rootserver
simple_elf := $(BUILD_DIR)/simple

.PHONY: build
build:
	pushd simple && cargo build $(CARGO_FLAGS) && popd
	cp $(simple_elf) rootserver/simple
	pushd rootserver && cargo build $(CARGO_FLAGS) && popd
	cp $(rootserver_elf) kernel/rootserver
	pushd kernel && cargo build $(CARGO_FLAGS) && popd

.PHONY: clean
clean:
	$(RM) -rf $(BUILD_DIR)

.PHONY: run
run: build
	$(QEMU) $(QEMUFLAGS) -kernel $(kernel_elf)

.PHONY: gdb
gdb:
	$(GDB) -q -ex "source ./gdbinit"

.PHONY: test-smoke
test-smoke:
	./scripts/qemu_smoke.sh

.PHONY: test
test: test-smoke
