import React, { useCallback, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import { invoke } from '@tauri-apps/api/tauri';

function App() {
	const [files, setFiles] = useState([]);

	const onDrop = useCallback((acceptedFiles) => {
		const newFiles = acceptedFiles.map((file) => ({
			path: file.path,
			name: file.name,
			size: file.size,
			file, // Store the file object
		}));
		setFiles((prevFiles) => [...prevFiles, ...newFiles]);
	}, []);

	const { getRootProps, getInputProps } = useDropzone({ onDrop });

	const handleUpload = () => {
		console.log('Upload button clicked');
		files.forEach((fileObj) => {
			const { file } = fileObj;
			const reader = new FileReader();

			reader.onload = () => {
				const binaryStr = reader.result;
				console.log(`Sending file: ${file.name}, size: ${file.size} bytes`);
				// Send the file content to the backend
				invoke('process_file', {
					fileName: file.name,
					fileContent: Array.from(new Uint8Array(binaryStr)),
				})
					.then((response) => console.log(response))
					.catch((error) => console.error(error));
			};

			reader.readAsArrayBuffer(file);
		});
	};

	return (
		<div className="App">
			<h1>File Upload</h1>
			<div
				{...getRootProps()}
				style={{
					border: '2px dashed #888',
					padding: '20px',
					textAlign: 'center',
				}}>
				<input {...getInputProps()} />
				<p>Drag & drop a file here, or click to select a file</p>
			</div>
			<div>
				<h2>Selected Files</h2>
				<ul>
					{files.map((file, index) => (
						<li key={index}>
							{file.name} - {(file.size / 1024).toFixed(2)} KB
						</li>
					))}
				</ul>
			</div>
			<button
				onClick={handleUpload}
				style={{ marginTop: '20px', padding: '10px 20px' }}>
				Upload Files
			</button>
		</div>
	);
}

export default App;
