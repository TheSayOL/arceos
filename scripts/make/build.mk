# Main building script

include scripts/make/cargo.mk
include scripts/make/features.mk

ifeq ($(APP_TYPE), rust_std)
  include scripts/make/build_std.mk
else
build_std:
endif

ifeq ($(APP_TYPE), c)
  include scripts/make/build_c.mk
else
  rust_package := $(shell cat $(APP)/Cargo.toml | sed -n 's/^name = "\([a-z0-9A-Z_\-]*\)"/\1/p')
  rust_target_dir := $(CURDIR)/target/$(TARGET)/$(MODE)
  rust_elf := $(rust_target_dir)/$(rust_package)
endif

ifeq ($(filter $(MAKECMDGOALS),build build_std run debug),$(MAKECMDGOALS))
  ifneq ($(V),)
    $(info APP: "$(APP)")
    $(info APP_TYPE: "$(APP_TYPE)")
    $(info FEATURES: "$(FEATURES)")
    $(info AX_FEAT: "$(AX_FEAT)")
    $(info LIB_FEAT: "$(LIB_FEAT)")
    $(info APP_FEAT: "$(APP_FEAT)")
  endif
  ifeq ($(APP_TYPE), c)
    $(if $(V), $(info CFLAGS: "$(CFLAGS)") $(info LDFLAGS: "$(LDFLAGS)"))
  else
    $(if $(V), $(info RUSTFLAGS: "$(RUSTFLAGS)"))
    export RUSTFLAGS
  endif
else ifneq ($(filter $(MAKECMDGOALS),doc doc_check_missing),)
  $(if $(V), $(info RUSTDOCFLAGS: "$(RUSTDOCFLAGS)"))
  export RUSTDOCFLAGS
endif

_cargo_build: build_std
	@printf "    $(GREEN_C)Building$(END_C) App: $(APP_NAME), Arch: $(ARCH), Platform: $(PLATFORM), App type: $(APP_TYPE)\n"
ifeq ($(APP_TYPE), rust)
	$(call cargo_rustc,--manifest-path $(APP)/Cargo.toml,$(AX_FEAT) $(LIB_FEAT) $(APP_FEAT))
	@cp $(rust_elf) $(OUT_ELF)
else ifeq ($(APP_TYPE), rust_std)
#	force rebuild the app to link the up-to-date std library
	@rm -rf $(rust_target_dir)/deps
	$(call cargo_rustc,--manifest-path $(APP)/Cargo.toml,$(APP_FEAT))
	@cp $(rust_elf) $(OUT_ELF)
else ifeq ($(APP_TYPE), c)
	$(call cargo_rustc,-p libax --crate-type staticlib,$(AX_FEAT) $(LIB_FEAT))
endif

$(OUT_DIR):
	$(call run_cmd,mkdir,-p $@)

$(OUT_BIN): _cargo_build $(OUT_ELF)
	$(call run_cmd,$(OBJCOPY),$(OUT_ELF) --strip-all -O binary $@)

.PHONY: _cargo_build build_std
