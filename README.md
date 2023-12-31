# socks5_over_tls
This is a tunnel forwarding program based on socks5 protocol + tls using rust.

Please compile the program and run the agent on the local computer and the server on the remote machine. Please replace the example.com domain name and port with your domain name. Note: The certificate here cannot be a self-signed certificate because the program defaults to using trusted certificate


send request:
application--> local socks5--> tls tunnel--> remote machine--> target website

response to request:
Target website-->remote machine -->tls tunnel -->local socks5--> application
