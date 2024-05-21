import base64
import os
import requests
import json
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from bs4 import BeautifulSoup

def convert_file_to_base64(original_file_path, base64_file_path):
    # Ensure the directory for the output file exists
    output_directory = os.path.dirname(base64_file_path)
    if not os.path.exists(output_directory):
        os.makedirs(output_directory)
    
    # Open the original file in binary mode
    with open(original_file_path, 'rb') as original_file:
        # Read the file's content
        file_content = original_file.read()
        
        # Encode the file content to base64
        base64_encoded_data = base64.b64encode(file_content)
        
        # Convert the bytes object to a string
        base64_encoded_string = base64_encoded_data.decode('utf-8')
        
    # Write the base64 string to the new file
    with open(base64_file_path, 'w') as base64_file:
        base64_file.write(base64_encoded_string)

def split_file_to_parts(base64_file_path, part_size_mb=1):
    part_size = part_size_mb * 1024 * 1024  # Convert MB to bytes
    output_directory = os.path.dirname(base64_file_path)
    
    # Read the base64-encoded content from the file
    with open(base64_file_path, 'r') as base64_file:
        base64_content = base64_file.read()
        
    part_files = []
    # Split the content into chunks of the specified size
    for i in range(0, len(base64_content), part_size):
        part_content = base64_content[i:i + part_size]
        part_file_path = os.path.join(output_directory, f'part-{i // part_size + 1}.txt')
        
        # Write each chunk to a separate file
        with open(part_file_path, 'w') as part_file:
            part_file.write(part_content)
        
        part_files.append(part_file_path)
    
    # Remove the original base64 file
    os.remove(base64_file_path)
    
    return part_files

def upload_part(file_path):
    with open(file_path, 'r') as file:
        part_content = file.read()
    
    data = {
        'lang': 'text',
        'text': part_content,
        'expire': '-1',
        'password': '',
        'title': ''
    }
    
    response = requests.post('https://pst.innomi.net/paste/new', data=data)
    
    print(f"Request to https://pst.innomi.net/paste/new with part content of length {len(part_content)}")
    print(f"Response Status Code: {response.status_code}")
    print(f"Response Text: {response.text[:200]}")  # Print first 200 characters of the response for inspection
    
    if response.status_code == 200:
        soup = BeautifulSoup(response.text, 'html.parser')
        title_tag = soup.find('title')
        if title_tag:
            title = title_tag.get_text().split(" - ")[0]  # Extract the relevant part before " - Ghostbin"
            return title
    else:
        print(f"Failed to upload {file_path}: {response.status_code}")
    return None

def save_links_to_json(links, json_file_path):
    formatted_links = [{"part-{}".format(i + 1): link} for i, link in enumerate(links)]
    with open(json_file_path, 'w') as json_file:
        json.dump(formatted_links, json_file, indent=4)

def clear_output_directory(output_directory):
    for file_name in os.listdir(output_directory):
        file_path = os.path.join(output_directory, file_name)
        try:
            if os.path.isfile(file_path):
                os.unlink(file_path)
        except Exception as e:
            print(f"Error deleting file {file_path}: {e}")

def main():
    original_file_path = 'video.mp4'  # Replace with your file's path
    base64_file_path = 'output/video64.txt'  # Note the relative path without leading slash
    json_file_path = 'output/response.json'  # Path to save the response JSON file
    output_directory = os.path.dirname(base64_file_path)
    
    # Clear the output directory
    clear_output_directory(output_directory)
    
    convert_file_to_base64(original_file_path, base64_file_path)
    print(f"Base64-encoded file saved to: {base64_file_path}")
    
    part_files = split_file_to_parts(base64_file_path, part_size_mb=5)
    print(f"Base64-encoded file split into 5MB parts and saved in the output directory.")
    print(f"The original base64-encoded file has been removed.")
    
    links = []
    with ThreadPoolExecutor(max_workers=10) as executor:
        futures = []
        for part_file in part_files:
            futures.append(executor.submit(upload_part, part_file))
            #time.sleep(0.5)  # Wait for 0.5 seconds between requests
        
        for future in as_completed(futures):
            link = future.result()
            if link:
                links.append(link)
    
    # Clear the output directory before saving response.json
    clear_output_directory(output_directory)
    
    save_links_to_json(links, json_file_path)
    print(f"All links have been saved to: {json_file_path}")

if __name__ == "__main__":
    main()
