SOURCES = matrix-88x88.chip.c
TARGET  = matrix-88x88.chip.wasm

all: $(TARGET)

$(TARGET): $(SOURCES) wokwi-api.h
	docker run --rm -v $(CURDIR):/src -w /src wokwi/builder-clang-wasm \
		clang --target=wasm32-unknown-wasi --sysroot /opt/wasi-libc -nostartfiles -Wl,--import-memory -Wl,--export-table -Wl,--no-entry -Werror -o $(TARGET) $(SOURCES)

clean:
	rm -f $(TARGET)
