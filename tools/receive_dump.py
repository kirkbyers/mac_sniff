from serial import Serial
import argparse
import os
import time
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description='Receive SPIFFS dump from ESP32')
    parser.add_argument('port', help='Serial port (e.g., /dev/ttyUSB0 or COM3)')
    parser.add_argument('--baud', type=int, default=115200, help='Baud rate (default: 115200)')
    parser.add_argument('--output', '-o', default='./dump', help='Output directory (default: ./dump)')
    
    args = parser.parse_args()
    
    # Create output directory if it doesn't exist
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"Opening serial port {args.port} at {args.baud} baud")
    print(f"Files will be saved to {output_dir.absolute()}")
    print("Waiting for data... (press Ctrl+C to abort)")
    
    with Serial(args.port, args.baud, timeout=1) as ser:
        receiving = False
        current_file = None
        file_path = None
        file_size = 0
        current_data = bytearray()
        
        while True:
            line = ser.readline().decode('utf-8', errors='ignore').strip()
            if not line:
                continue
                
            if line == "MAC_SNIFF_DUMP_BEGIN":
                print("Transfer started")
                receiving = True
                continue
                
            if not receiving:
                continue
                
            if line == "MAC_SNIFF_DUMP_END":
                print("Transfer completed")
                total_bytes = 0
                if line.startswith("TOTAL_BYTES:"):
                    total_bytes = int(line.split(":")[1])
                print(f"Received {total_bytes} bytes total")
                break
                
            if line.startswith("NUM_FILES:"):
                num_files = int(line.split(":")[1])
                print(f"Expecting {num_files} files")
                continue
                
            if line.startswith("FILE_BEGIN:"):
                file_path = line.split(":", 1)[1]
                print(f"Receiving: {file_path}")
                # Extract just the filename part
                base_filename = os.path.basename(file_path)
                current_file = output_dir / base_filename
                current_data = bytearray()
                continue
                
            if line.startswith("FILE_SIZE:"):
                file_size = int(line.split(":")[1])
                print(f"File size: {file_size} bytes")
                continue
                
            if line.startswith("CHUNK:"):
                hex_data = line[6:]  # Remove "CHUNK:" prefix
                # Convert hex string back to binary
                for i in range(0, len(hex_data), 2):
                    if i+1 < len(hex_data):
                        byte = int(hex_data[i:i+2], 16)
                        current_data.append(byte)
                continue
                
            if line == "FILE_END" and current_file and current_data:
                # Save the file
                with open(current_file, 'wb') as f:
                    f.write(current_data)
                print(f"Saved: {current_file} ({len(current_data)} bytes)")
                current_file = None
                current_data = bytearray()
                continue

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nAborted by user")
    except Exception as e:
        print(f"Error: {e}")