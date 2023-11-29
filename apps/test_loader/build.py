import os

APP_NAMES = ['a', 'b']
APPS_BIN = 'apps.bin'
PAYLOAD_DIR = '../../payload'
HEADER_NAMES = ['magic', 'app_off', 'app_size']
HEADER_ALIGN = 8

def main():

    # compile
    os.system("mkdir -p build")
    for name in APP_NAMES:
        os.system(f"riscv64-linux-musl-gcc c/{name}.c -o build/{name}")

    # read apps' data
    app_data = []
    for name in APP_NAMES:
        file = open(f'build/{name}','rb')
        data = file.read()
        app_data.append(data)

    # create a temp file to write app data
    file = open(f'build/temp', 'wb+')
    # write data for each app  
    for i, data in enumerate(app_data):
        # write headers, with align for 8 bytes
        file.write("UniKernl".encode()) # magic 
        file.write((len(HEADER_NAMES) * HEADER_ALIGN).to_bytes(HEADER_ALIGN, 'little')) # app_off: header size
        file.write(os.path.getsize(f'build/{APP_NAMES[i]}').to_bytes(HEADER_ALIGN, 'little')) # app_size: data size
        # wrte app data
        file.write(data)
    file.close() # save 

    # extend temp file to 32M, and rename it 
    os.system(f'dd if=build/temp of=./{APPS_BIN} bs=32M conv=sync')

    # mkdir and mv 
    os.system(f"mkdir -p {PAYLOAD_DIR}")
    os.system(f"cp {APPS_BIN} {PAYLOAD_DIR}/{APPS_BIN}")

    # clean
    # os.remove('temp')
    # for name in APP_NAMES:
    #     os.remove(f"{name}")


if __name__ == "__main__":
    main()