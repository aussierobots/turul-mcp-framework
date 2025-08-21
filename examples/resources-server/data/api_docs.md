# API Documentation

## Authentication

All API requests require a valid API key in the Authorization header:
```
Authorization: Bearer your-api-key-here
```

## Base URL
```
https://api.example.com/v1
```

## Rate Limits
- 1000 requests per hour for authenticated users
- 100 requests per hour for unauthenticated users

## Endpoints

### GET /users

Retrieve a list of users.

**Parameters:**
- `page` (integer, optional): Page number (default: 1)
- `limit` (integer, optional): Items per page (default: 20, max: 100)
- `filter` (string, optional): Filter by username or email

**Response:**
```json
{
  "users": [
    {
      "id": 1,
      "username": "john_doe",
      "email": "john@example.com",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 150,
    "pages": 8
  }
}
```

### POST /users

Create a new user.

**Request Body:**
```json
{
  "username": "new_user",
  "email": "user@example.com",
  "password": "secure_password"
}
```

**Response:**
```json
{
  "id": 151,
  "username": "new_user",
  "email": "user@example.com",
  "created_at": "2024-01-15T10:30:00Z"
}
```

### GET /users/{id}

Retrieve a specific user by ID.

**Parameters:**
- `id` (integer, required): User ID

**Response:**
```json
{
  "id": 1,
  "username": "john_doe",
  "email": "john@example.com",
  "created_at": "2024-01-01T00:00:00Z",
  "profile": {
    "first_name": "John",
    "last_name": "Doe",
    "bio": "Software developer with 5 years experience"
  }
}
```

### PUT /users/{id}

Update a user.

**Parameters:**
- `id` (integer, required): User ID

**Request Body:**
```json
{
  "email": "newemail@example.com",
  "profile": {
    "first_name": "John",
    "last_name": "Smith",
    "bio": "Senior software developer with 6 years experience"
  }
}
```

### DELETE /users/{id}

Delete a user.

**Parameters:**
- `id` (integer, required): User ID

**Response:**
```json
{
  "message": "User deleted successfully"
}
```

## Error Handling

### Error Response Format
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "The request data is invalid",
    "details": {
      "field": "email",
      "message": "Email format is invalid"
    }
  }
}
```

### Common Error Codes
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `429` - Too Many Requests
- `500` - Internal Server Error

## SDKs and Libraries

### JavaScript/Node.js
```bash
npm install @company/api-client
```

```javascript
const { ApiClient } = require('@company/api-client');

const client = new ApiClient({
  apiKey: 'your-api-key',
  baseUrl: 'https://api.example.com/v1'
});

const users = await client.users.list({ page: 1, limit: 50 });
```

### Python
```bash
pip install company-api-client
```

```python
from company_api import Client

client = Client(api_key='your-api-key')
users = client.users.list(page=1, limit=50)
```

### cURL Examples

```bash
# List users
curl -H "Authorization: Bearer your-api-key" \
     https://api.example.com/v1/users

# Create user
curl -X POST \
     -H "Authorization: Bearer your-api-key" \
     -H "Content-Type: application/json" \
     -d '{"username": "newuser", "email": "user@example.com", "password": "secure123"}' \
     https://api.example.com/v1/users
```