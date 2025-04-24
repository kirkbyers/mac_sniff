import os
import sys
import argparse
from datetime import datetime

def mac_to_string(mac_bytes):
    """Convert a 6-byte MAC address to a human-readable string."""
    return ":".join([f"{b:02x}" for b in mac_bytes])

def process_binary_file(input_file, output_file):
    """Process a binary file containing 6-byte MAC addresses and write them to a text file."""
    try:
        with open(input_file, 'rb') as f:
            data = f.read()
        
        # Each MAC address is 6 bytes
        mac_count = len(data) // 6
        
        with open(output_file, 'w') as out:
            out.write(f"# MAC addresses extracted from {os.path.basename(input_file)}\n")
            out.write(f"# Extracted on {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
            out.write(f"# Total MAC addresses: {mac_count}\n\n")
            
            for i in range(0, len(data), 6):
                if i + 6 <= len(data):  # Ensure we have a complete MAC address
                    mac_bytes = data[i:i+6]
                    mac_str = mac_to_string(mac_bytes)
                    out.write(f"{mac_str}\n")
        
        print(f"Processed {mac_count} MAC addresses from {input_file}")
        return mac_count
    except Exception as e:
        print(f"Error processing {input_file}: {e}")
        return 0

def main():
    parser = argparse.ArgumentParser(description='Convert MAC address binary dumps to readable text files')
    parser.add_argument('input', help='Input binary file or directory containing binary files')
    parser.add_argument('-o', '--output-dir', help='Output directory for text files', default='converted')
    args = parser.parse_args()
    
    # Create output directory if it doesn't exist
    if not os.path.exists(args.output_dir):
        os.makedirs(args.output_dir)
    
    total_files = 0
    total_macs = 0
    
    if os.path.isdir(args.input):
        # Process all .bin files in the directory
        for filename in os.listdir(args.input):
            if filename.endswith('.bin'):
                input_path = os.path.join(args.input, filename)
                output_path = os.path.join(args.output_dir, f"{os.path.splitext(filename)[0]}.txt")
                macs = process_binary_file(input_path, output_path)
                if macs > 0:
                    total_files += 1
                    total_macs += macs
    else:
        # Process a single file
        output_filename = f"{os.path.splitext(os.path.basename(args.input))[0]}.txt"
        output_path = os.path.join(args.output_dir, output_filename)
        macs = process_binary_file(args.input, output_path)
        if macs > 0:
            total_files += 1
            total_macs += macs
    
    print(f"Conversion complete: {total_files} files processed, {total_macs} MAC addresses extracted")
    print(f"Output files saved to {os.path.abspath(args.output_dir)}")

if __name__ == "__main__":
    main()