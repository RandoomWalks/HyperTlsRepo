### **Explaining the TLS Shutdown Issue**

#### **Overview of the Problem**
The **proxy server** is experiencing issues during the **shutdown phase** of client connections, specifically for **TLS-secured connections**. The goal of a **graceful shutdown** is to ensure:
1. All data in transit has been fully sent to the client.
2. The client is notified that the server is closing the connection.
3. Both the server and client agree that the connection can be safely closed.

However, in certain cases, the shutdown process fails due to timing problems, client misbehavior, or network issues, resulting in:
- **Data loss**: The client doesn’t receive all the data before the connection is closed.
- **Hanging connections**: The server waits too long for the client to respond to the shutdown, wasting resources.
- **Errors**: The server force-closes the connection prematurely, leading to incomplete communication.



### **Proxy Server Architecture**

```plaintext
                                    +---------------------------+
                                    |      Client Request       |
                                    |   (e.g., HTTPS browser)   |
                                    +------------+--------------+
                                                 |
                                                 v
+------------------------------------------------+---------------------------------+
|                                 Proxy Server (TLS)                              |
|---------------------------------------------------------------------------------|
|                                                                                 |
|   +-------------------+            +---------------------+                      |
|   | TLS Handshake     |            | Diagnostics Module  |                      |
|   | (Tokio Rustls)    |<---------->| Connection Lifecycle |                      |
|   +-------------------+            | - States (Created,   |                      |
|                                     |   Handshake, etc.)   |                      |
|                                     | - Metrics (Durations,|                      |
|                                     |   Buffers, etc.)     |                      |
|                                     +---------------------+                      |
|                                                                                 |
|  +--------------------+            +---------------------+                      |
|  | HTTP Handler       |<---------->| Shutdown Handling   |                      |
|  | (Hyper)            |            | - Graceful Sequence |                      |
|  | - Parses requests  |            | - Logs & Metrics    |                      |
|  | - Proxies traffic  |            +---------------------+                      |
|  +--------------------+                                                   +-----+
|                                                                                 |
+------------------------------------------------+---------------------------------+
                                                 |
                                                 v
                                   +------------------------------+
                                   | Backend Server (e.g., API)   |
                                   +------------------------------+
```

---

### **TLS Connection Lifecycle**

```plaintext
+---------------------+          +---------------------+           +------------------+
|     Connection      |  ----->  |     TLS Handshake   |   ----->  |   Data Transfer  |
|     (Created)       |          |   (Handshake Done)  |           |                  |
+---------------------+          +---------------------+           +------------------+
                                         |
                                         v
                               +-----------------------+
                               |   Shutdown Initiated  |
                               +-----------------------+
                                         |
                                         v
                          +---------------------------+
                          |   Buffer Flushed          |
                          +---------------------------+
                                         |
                                         v
                        +-----------------------------+
                        |   TLS Close Notify Sent     |
                        +-----------------------------+
                                         |
                                         v
                           +-----------------------+
                           |   TCP Socket Closed   |
                           +-----------------------+
```

---

### **Data and Diagnostics Flow**

```plaintext
+-------------------+        +-----------------------+        +--------------------------+
|  Incoming Client  |  --->  |  Proxy Server (TLS)  |  --->  |    Backend Server/API     |
|  Request          |        |   - Accept TLS       |        |    Processes Request      |
|                   |        |   - Parse HTTP       |        +--------------------------+
|                   |        |   - Proxy to Backend |
|                   |        |                      |        +--------------------------+
|                   |        |   Diagnostics        |  <---  |  Backend Server/API       |
|                   |        |   - States/Logs      |        |  Sends Response           |
|                   |        |   - Metrics          |        +--------------------------+
|                   |        |   - Shutdown Details |
+-------------------+        +-----------------------+        +--------------------------+
```

---

### **Shutdown Sequence with Diagnostics**

```plaintext
+-------------------------------+
|   Shutdown Process Begins     |
+-------------------------------+
              |
              v
+-------------------------------+
|   Log Connection State        |   e.g., "State: Data Transfer"
+-------------------------------+
              |
              v
+-------------------------------+
|   Flush Buffers               |   e.g., "Buffer Size: 1024 bytes"
+-------------------------------+
              |
              v
+-------------------------------+
|   Send TLS Close Notify       |   e.g., "TLS Close Notify Sent"
+-------------------------------+
              |
              v
+-------------------------------+
|   Close TCP Socket            |   e.g., "Socket Closed Cleanly"
+-------------------------------+
              |
              v
+-------------------------------+
|   Log Shutdown Metrics        |   e.g., "Shutdown Duration: 120ms"
+-------------------------------+
```

---

### **Key Logging and Metrics**

```plaintext
[DEBUG] Connection ID: abc123
[INFO] State: Data Transfer
[DEBUG] Buffer State: Pending Writes = 1024 bytes
[INFO] Shutdown: Flush Completed
[INFO] Shutdown: TLS Close Notify Sent
[INFO] Shutdown: TCP Socket Closed
[INFO] Metrics: Shutdown Duration = 150ms
```