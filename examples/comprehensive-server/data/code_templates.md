# Development Team Code Templates and Standards

## Overview

This document provides comprehensive code templates, standards, and best practices for development teams. These templates help maintain consistency, reduce boilerplate, and accelerate development across all projects and technologies.

## Backend Service Templates

### 1. REST API Service Template (Node.js/Express)

#### Basic API Structure
```javascript
// server.js - Main server entry point
const express = require('express');
const cors = require('cors');
const helmet = require('helmet');
const rateLimit = require('express-rate-limit');
const { v4: uuidv4 } = require('uuid');

const app = express();
const PORT = process.env.PORT || 3000;

// Security middleware
app.use(helmet());
app.use(cors({
  origin: process.env.ALLOWED_ORIGINS?.split(',') || ['http://localhost:3000'],
  credentials: true
}));

// Rate limiting
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // limit each IP to 100 requests per windowMs
  message: 'Too many requests from this IP, please try again later.'
});
app.use('/api/', limiter);

// Body parsing middleware
app.use(express.json({ limit: '10mb' }));
app.use(express.urlencoded({ extended: true }));

// Request ID middleware
app.use((req, res, next) => {
  req.id = uuidv4();
  res.set('X-Request-ID', req.id);
  next();
});

// Health check endpoint
app.get('/health', (req, res) => {
  res.status(200).json({
    status: 'healthy',
    timestamp: new Date().toISOString(),
    service: process.env.SERVICE_NAME || 'api-service',
    version: process.env.SERVICE_VERSION || '1.0.0'
  });
});

// API routes
app.use('/api/v1', require('./routes'));

// Global error handler
app.use((err, req, res, next) => {
  console.error(`[${req.id}] Error:`, err);
  
  if (err.status) {
    return res.status(err.status).json({
      error: err.message,
      requestId: req.id
    });
  }
  
  res.status(500).json({
    error: 'Internal server error',
    requestId: req.id
  });
});

// 404 handler
app.use('*', (req, res) => {
  res.status(404).json({
    error: 'Route not found',
    requestId: req.id
  });
});

app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});

module.exports = app;
```

#### Controller Template
```javascript
// controllers/userController.js
const { validationResult } = require('express-validator');
const userService = require('../services/userService');
const { ApiError } = require('../utils/errors');

class UserController {
  async createUser(req, res, next) {
    try {
      // Validate input
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        throw new ApiError(400, 'Validation failed', errors.array());
      }

      const userData = req.body;
      const user = await userService.createUser(userData);
      
      res.status(201).json({
        success: true,
        data: user,
        message: 'User created successfully'
      });
    } catch (error) {
      next(error);
    }
  }

  async getUserById(req, res, next) {
    try {
      const { id } = req.params;
      const user = await userService.getUserById(id);
      
      if (!user) {
        throw new ApiError(404, 'User not found');
      }

      res.status(200).json({
        success: true,
        data: user
      });
    } catch (error) {
      next(error);
    }
  }

  async updateUser(req, res, next) {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        throw new ApiError(400, 'Validation failed', errors.array());
      }

      const { id } = req.params;
      const updateData = req.body;
      
      const updatedUser = await userService.updateUser(id, updateData);
      
      res.status(200).json({
        success: true,
        data: updatedUser,
        message: 'User updated successfully'
      });
    } catch (error) {
      next(error);
    }
  }

  async deleteUser(req, res, next) {
    try {
      const { id } = req.params;
      await userService.deleteUser(id);
      
      res.status(200).json({
        success: true,
        message: 'User deleted successfully'
      });
    } catch (error) {
      next(error);
    }
  }

  async listUsers(req, res, next) {
    try {
      const { page = 1, limit = 10, sortBy = 'createdAt', sortOrder = 'desc' } = req.query;
      
      const result = await userService.listUsers({
        page: parseInt(page),
        limit: parseInt(limit),
        sortBy,
        sortOrder
      });
      
      res.status(200).json({
        success: true,
        data: result.users,
        pagination: {
          currentPage: result.currentPage,
          totalPages: result.totalPages,
          totalItems: result.totalItems,
          itemsPerPage: result.itemsPerPage
        }
      });
    } catch (error) {
      next(error);
    }
  }
}

module.exports = new UserController();
```

#### Service Template
```javascript
// services/userService.js
const bcrypt = require('bcrypt');
const { User } = require('../models');
const { ApiError } = require('../utils/errors');

class UserService {
  async createUser(userData) {
    const { email, password, ...otherData } = userData;
    
    // Check if user already exists
    const existingUser = await User.findOne({ where: { email } });
    if (existingUser) {
      throw new ApiError(409, 'User with this email already exists');
    }

    // Hash password
    const hashedPassword = await bcrypt.hash(password, 12);

    // Create user
    const user = await User.create({
      email,
      password: hashedPassword,
      ...otherData
    });

    // Remove password from response
    const { password: _, ...userWithoutPassword } = user.toJSON();
    return userWithoutPassword;
  }

  async getUserById(id) {
    const user = await User.findByPk(id, {
      attributes: { exclude: ['password'] }
    });
    return user;
  }

  async updateUser(id, updateData) {
    const user = await User.findByPk(id);
    if (!user) {
      throw new ApiError(404, 'User not found');
    }

    // Hash password if provided
    if (updateData.password) {
      updateData.password = await bcrypt.hash(updateData.password, 12);
    }

    await user.update(updateData);
    
    // Remove password from response
    const { password: _, ...userWithoutPassword } = user.toJSON();
    return userWithoutPassword;
  }

  async deleteUser(id) {
    const user = await User.findByPk(id);
    if (!user) {
      throw new ApiError(404, 'User not found');
    }

    await user.destroy();
  }

  async listUsers(options) {
    const { page, limit, sortBy, sortOrder } = options;
    const offset = (page - 1) * limit;

    const { count, rows } = await User.findAndCountAll({
      attributes: { exclude: ['password'] },
      limit,
      offset,
      order: [[sortBy, sortOrder.toUpperCase()]],
    });

    return {
      users: rows,
      totalItems: count,
      currentPage: page,
      totalPages: Math.ceil(count / limit),
      itemsPerPage: limit
    };
  }
}

module.exports = new UserService();
```

### 2. Database Model Template (Sequelize)

```javascript
// models/User.js
const { DataTypes } = require('sequelize');
const sequelize = require('../config/database');

const User = sequelize.define('User', {
  id: {
    type: DataTypes.UUID,
    defaultValue: DataTypes.UUIDV4,
    primaryKey: true,
  },
  email: {
    type: DataTypes.STRING,
    allowNull: false,
    unique: true,
    validate: {
      isEmail: true
    }
  },
  password: {
    type: DataTypes.STRING,
    allowNull: false,
    validate: {
      len: [8, 128]
    }
  },
  firstName: {
    type: DataTypes.STRING,
    allowNull: false,
    validate: {
      len: [2, 50]
    }
  },
  lastName: {
    type: DataTypes.STRING,
    allowNull: false,
    validate: {
      len: [2, 50]
    }
  },
  role: {
    type: DataTypes.ENUM('user', 'admin', 'moderator'),
    defaultValue: 'user'
  },
  isActive: {
    type: DataTypes.BOOLEAN,
    defaultValue: true
  },
  lastLoginAt: {
    type: DataTypes.DATE
  },
  emailVerifiedAt: {
    type: DataTypes.DATE
  }
}, {
  tableName: 'users',
  timestamps: true,
  paranoid: true, // Soft deletes
  indexes: [
    {
      fields: ['email']
    },
    {
      fields: ['role']
    },
    {
      fields: ['isActive']
    }
  ]
});

// Instance methods
User.prototype.toJSON = function() {
  const values = { ...this.get() };
  delete values.password;
  return values;
};

// Class methods
User.findByEmail = function(email) {
  return this.findOne({
    where: { email }
  });
};

module.exports = User;
```

## Frontend Component Templates

### 1. React Component Template (TypeScript)

#### Functional Component with Hooks
```typescript
// components/UserProfile/UserProfile.tsx
import React, { useState, useEffect, useCallback } from 'react';
import { User, ApiResponse } from '../../types';
import { userApi } from '../../services/api';
import { useNotification } from '../../hooks/useNotification';
import { LoadingSpinner } from '../LoadingSpinner';
import { Button } from '../Button';
import styles from './UserProfile.module.css';

interface UserProfileProps {
  userId: string;
  onUserUpdate?: (user: User) => void;
  className?: string;
}

export const UserProfile: React.FC<UserProfileProps> = ({
  userId,
  onUserUpdate,
  className
}) => {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isEditing, setIsEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const { showNotification } = useNotification();

  const fetchUser = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      
      const response: ApiResponse<User> = await userApi.getUser(userId);
      setUser(response.data);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to fetch user';
      setError(errorMessage);
      showNotification('Error loading user profile', 'error');
    } finally {
      setIsLoading(false);
    }
  }, [userId, showNotification]);

  const handleUpdateUser = async (userData: Partial<User>) => {
    try {
      const response: ApiResponse<User> = await userApi.updateUser(userId, userData);
      setUser(response.data);
      setIsEditing(false);
      
      onUserUpdate?.(response.data);
      showNotification('Profile updated successfully', 'success');
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to update user';
      showNotification(errorMessage, 'error');
    }
  };

  useEffect(() => {
    if (userId) {
      fetchUser();
    }
  }, [userId, fetchUser]);

  if (isLoading) {
    return <LoadingSpinner />;
  }

  if (error || !user) {
    return (
      <div className={`${styles.error} ${className}`}>
        <p>Unable to load user profile</p>
        <Button onClick={fetchUser} variant="secondary">
          Retry
        </Button>
      </div>
    );
  }

  return (
    <div className={`${styles.container} ${className}`}>
      <div className={styles.header}>
        <h2>User Profile</h2>
        <Button
          onClick={() => setIsEditing(!isEditing)}
          variant="outline"
        >
          {isEditing ? 'Cancel' : 'Edit'}
        </Button>
      </div>

      {isEditing ? (
        <UserEditForm
          user={user}
          onSave={handleUpdateUser}
          onCancel={() => setIsEditing(false)}
        />
      ) : (
        <UserDisplayInfo user={user} />
      )}
    </div>
  );
};
```

#### Custom Hook Template
```typescript
// hooks/useApi.ts
import { useState, useCallback } from 'react';

interface UseApiState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

interface UseApiOptions {
  onSuccess?: (data: any) => void;
  onError?: (error: string) => void;
}

export function useApi<T = any>(options: UseApiOptions = {}) {
  const [state, setState] = useState<UseApiState<T>>({
    data: null,
    loading: false,
    error: null
  });

  const execute = useCallback(async (apiCall: Promise<T>) => {
    setState(prev => ({ ...prev, loading: true, error: null }));

    try {
      const data = await apiCall;
      setState({ data, loading: false, error: null });
      options.onSuccess?.(data);
      return data;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'An error occurred';
      setState(prev => ({ ...prev, loading: false, error: errorMessage }));
      options.onError?.(errorMessage);
      throw error;
    }
  }, [options]);

  const reset = useCallback(() => {
    setState({ data: null, loading: false, error: null });
  }, []);

  return {
    ...state,
    execute,
    reset
  };
}
```

### 2. Vue.js Component Template

```vue
<!-- components/UserDashboard.vue -->
<template>
  <div class="user-dashboard">
    <header class="dashboard-header">
      <h1>Welcome, {{ user?.firstName || 'User' }}</h1>
      <div class="header-actions">
        <button @click="refreshData" :disabled="loading" class="btn-refresh">
          <RefreshIcon :class="{ spinning: loading }" />
          Refresh
        </button>
      </div>
    </header>

    <div class="dashboard-grid">
      <div class="stats-section">
        <StatsCard
          v-for="stat in stats"
          :key="stat.id"
          :title="stat.title"
          :value="stat.value"
          :trend="stat.trend"
          :icon="stat.icon"
        />
      </div>

      <div class="activity-section">
        <h2>Recent Activity</h2>
        <ActivityList
          :activities="activities"
          :loading="activitiesLoading"
          @load-more="loadMoreActivities"
        />
      </div>

      <div class="notifications-section">
        <NotificationCenter
          :notifications="notifications"
          @mark-read="markNotificationRead"
          @dismiss="dismissNotification"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useUserStore } from '../stores/userStore';
import { useNotificationStore } from '../stores/notificationStore';
import { userApi, activityApi } from '../services/api';
import type { User, Activity, DashboardStats } from '../types';

// Components
import StatsCard from './StatsCard.vue';
import ActivityList from './ActivityList.vue';
import NotificationCenter from './NotificationCenter.vue';
import RefreshIcon from './icons/RefreshIcon.vue';

// Props
interface Props {
  userId: string;
}

const props = defineProps<Props>();

// Stores
const userStore = useUserStore();
const notificationStore = useNotificationStore();

// Reactive state
const loading = ref(false);
const activitiesLoading = ref(false);
const stats = ref<DashboardStats[]>([]);
const activities = ref<Activity[]>([]);

// Computed
const user = computed(() => userStore.currentUser);
const notifications = computed(() => notificationStore.unreadNotifications);

// Methods
const fetchDashboardData = async () => {
  loading.value = true;
  try {
    const [statsResponse, activitiesResponse] = await Promise.all([
      userApi.getUserStats(props.userId),
      activityApi.getRecentActivities(props.userId, { limit: 10 })
    ]);

    stats.value = statsResponse.data;
    activities.value = activitiesResponse.data;
  } catch (error) {
    notificationStore.addNotification({
      type: 'error',
      message: 'Failed to load dashboard data',
      duration: 5000
    });
  } finally {
    loading.value = false;
  }
};

const loadMoreActivities = async () => {
  if (activitiesLoading.value) return;

  activitiesLoading.value = true;
  try {
    const response = await activityApi.getRecentActivities(props.userId, {
      limit: 10,
      offset: activities.value.length
    });

    activities.value.push(...response.data);
  } catch (error) {
    notificationStore.addNotification({
      type: 'error',
      message: 'Failed to load more activities'
    });
  } finally {
    activitiesLoading.value = false;
  }
};

const refreshData = () => {
  fetchDashboardData();
};

const markNotificationRead = (notificationId: string) => {
  notificationStore.markAsRead(notificationId);
};

const dismissNotification = (notificationId: string) => {
  notificationStore.removeNotification(notificationId);
};

// Lifecycle
onMounted(() => {
  fetchDashboardData();
});
</script>

<style scoped>
.user-dashboard {
  padding: 2rem;
  max-width: 1200px;
  margin: 0 auto;
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 2rem;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  grid-template-rows: auto auto;
  gap: 2rem;
}

.stats-section {
  grid-column: 1 / -1;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 1rem;
}

.activity-section,
.notifications-section {
  background: white;
  border-radius: 8px;
  padding: 1.5rem;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.btn-refresh {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.btn-refresh:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (max-width: 768px) {
  .dashboard-grid {
    grid-template-columns: 1fr;
  }
  
  .user-dashboard {
    padding: 1rem;
  }
}
</style>
```

## Testing Templates

### 1. Unit Test Template (Jest/TypeScript)

```typescript
// __tests__/services/userService.test.ts
import { userService } from '../../src/services/userService';
import { User } from '../../src/models/User';
import { ApiError } from '../../src/utils/errors';

// Mock dependencies
jest.mock('../../src/models/User');
jest.mock('bcrypt');

describe('UserService', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('createUser', () => {
    const mockUserData = {
      email: 'test@example.com',
      password: 'password123',
      firstName: 'John',
      lastName: 'Doe'
    };

    it('should create a new user successfully', async () => {
      // Arrange
      const mockUser = { 
        id: '123', 
        ...mockUserData, 
        password: 'hashedPassword',
        toJSON: () => ({ id: '123', ...mockUserData })
      };
      
      (User.findOne as jest.Mock).mockResolvedValue(null);
      (User.create as jest.Mock).mockResolvedValue(mockUser);

      // Act
      const result = await userService.createUser(mockUserData);

      // Assert
      expect(User.findOne).toHaveBeenCalledWith({
        where: { email: mockUserData.email }
      });
      expect(User.create).toHaveBeenCalledWith({
        ...mockUserData,
        password: expect.any(String) // Should be hashed
      });
      expect(result).not.toHaveProperty('password');
      expect(result.email).toBe(mockUserData.email);
    });

    it('should throw error if user already exists', async () => {
      // Arrange
      (User.findOne as jest.Mock).mockResolvedValue({ id: '123' });

      // Act & Assert
      await expect(userService.createUser(mockUserData))
        .rejects
        .toThrow(ApiError);
        
      expect(User.create).not.toHaveBeenCalled();
    });

    it('should handle database errors gracefully', async () => {
      // Arrange
      (User.findOne as jest.Mock).mockRejectedValue(new Error('Database error'));

      // Act & Assert
      await expect(userService.createUser(mockUserData))
        .rejects
        .toThrow('Database error');
    });
  });

  describe('getUserById', () => {
    it('should return user without password', async () => {
      // Arrange
      const mockUser = {
        id: '123',
        email: 'test@example.com',
        firstName: 'John',
        lastName: 'Doe'
      };
      
      (User.findByPk as jest.Mock).mockResolvedValue(mockUser);

      // Act
      const result = await userService.getUserById('123');

      // Assert
      expect(User.findByPk).toHaveBeenCalledWith('123', {
        attributes: { exclude: ['password'] }
      });
      expect(result).toEqual(mockUser);
    });

    it('should return null if user not found', async () => {
      // Arrange
      (User.findByPk as jest.Mock).mockResolvedValue(null);

      // Act
      const result = await userService.getUserById('nonexistent');

      // Assert
      expect(result).toBeNull();
    });
  });

  describe('deleteUser', () => {
    it('should delete user successfully', async () => {
      // Arrange
      const mockUser = {
        id: '123',
        destroy: jest.fn().mockResolvedValue(true)
      };
      
      (User.findByPk as jest.Mock).mockResolvedValue(mockUser);

      // Act
      await userService.deleteUser('123');

      // Assert
      expect(User.findByPk).toHaveBeenCalledWith('123');
      expect(mockUser.destroy).toHaveBeenCalled();
    });

    it('should throw error if user not found', async () => {
      // Arrange
      (User.findByPk as jest.Mock).mockResolvedValue(null);

      // Act & Assert
      await expect(userService.deleteUser('nonexistent'))
        .rejects
        .toThrow(ApiError);
    });
  });
});
```

### 2. Integration Test Template (API Testing)

```typescript
// __tests__/integration/userApi.test.ts
import request from 'supertest';
import app from '../../src/app';
import { setupTestDatabase, teardownTestDatabase } from '../helpers/database';
import { createTestUser, generateJwtToken } from '../helpers/auth';

describe('User API Integration Tests', () => {
  beforeAll(async () => {
    await setupTestDatabase();
  });

  afterAll(async () => {
    await teardownTestDatabase();
  });

  describe('POST /api/v1/users', () => {
    const validUserData = {
      email: 'newuser@example.com',
      password: 'SecurePass123!',
      firstName: 'Jane',
      lastName: 'Smith'
    };

    it('should create a new user with valid data', async () => {
      const response = await request(app)
        .post('/api/v1/users')
        .send(validUserData)
        .expect(201);

      expect(response.body).toMatchObject({
        success: true,
        data: {
          email: validUserData.email,
          firstName: validUserData.firstName,
          lastName: validUserData.lastName
        },
        message: 'User created successfully'
      });

      expect(response.body.data).not.toHaveProperty('password');
      expect(response.body.data).toHaveProperty('id');
      expect(response.body.data).toHaveProperty('createdAt');
    });

    it('should return 400 for invalid email format', async () => {
      const invalidData = { ...validUserData, email: 'invalid-email' };

      const response = await request(app)
        .post('/api/v1/users')
        .send(invalidData)
        .expect(400);

      expect(response.body).toMatchObject({
        error: 'Validation failed'
      });
    });

    it('should return 409 for duplicate email', async () => {
      // First, create a user
      await request(app)
        .post('/api/v1/users')
        .send(validUserData)
        .expect(201);

      // Try to create another user with same email
      const response = await request(app)
        .post('/api/v1/users')
        .send(validUserData)
        .expect(409);

      expect(response.body).toMatchObject({
        error: 'User with this email already exists'
      });
    });
  });

  describe('GET /api/v1/users/:id', () => {
    it('should return user data for valid ID', async () => {
      const testUser = await createTestUser();
      const token = generateJwtToken(testUser.id);

      const response = await request(app)
        .get(`/api/v1/users/${testUser.id}`)
        .set('Authorization', `Bearer ${token}`)
        .expect(200);

      expect(response.body).toMatchObject({
        success: true,
        data: {
          id: testUser.id,
          email: testUser.email,
          firstName: testUser.firstName,
          lastName: testUser.lastName
        }
      });

      expect(response.body.data).not.toHaveProperty('password');
    });

    it('should return 404 for non-existent user', async () => {
      const testUser = await createTestUser();
      const token = generateJwtToken(testUser.id);

      const response = await request(app)
        .get('/api/v1/users/non-existent-id')
        .set('Authorization', `Bearer ${token}`)
        .expect(404);

      expect(response.body).toMatchObject({
        error: 'User not found'
      });
    });

    it('should return 401 without authentication token', async () => {
      const testUser = await createTestUser();

      await request(app)
        .get(`/api/v1/users/${testUser.id}`)
        .expect(401);
    });
  });

  describe('PUT /api/v1/users/:id', () => {
    it('should update user data successfully', async () => {
      const testUser = await createTestUser();
      const token = generateJwtToken(testUser.id);
      const updateData = {
        firstName: 'UpdatedFirst',
        lastName: 'UpdatedLast'
      };

      const response = await request(app)
        .put(`/api/v1/users/${testUser.id}`)
        .set('Authorization', `Bearer ${token}`)
        .send(updateData)
        .expect(200);

      expect(response.body).toMatchObject({
        success: true,
        data: {
          id: testUser.id,
          firstName: updateData.firstName,
          lastName: updateData.lastName
        },
        message: 'User updated successfully'
      });
    });

    it('should not allow email updates to existing email', async () => {
      const user1 = await createTestUser({ email: 'user1@example.com' });
      const user2 = await createTestUser({ email: 'user2@example.com' });
      const token = generateJwtToken(user1.id);

      const response = await request(app)
        .put(`/api/v1/users/${user1.id}`)
        .set('Authorization', `Bearer ${token}`)
        .send({ email: user2.email })
        .expect(409);

      expect(response.body.error).toContain('already exists');
    });
  });

  describe('DELETE /api/v1/users/:id', () => {
    it('should delete user successfully', async () => {
      const testUser = await createTestUser();
      const token = generateJwtToken(testUser.id);

      const response = await request(app)
        .delete(`/api/v1/users/${testUser.id}`)
        .set('Authorization', `Bearer ${token}`)
        .expect(200);

      expect(response.body).toMatchObject({
        success: true,
        message: 'User deleted successfully'
      });

      // Verify user is actually deleted
      await request(app)
        .get(`/api/v1/users/${testUser.id}`)
        .set('Authorization', `Bearer ${token}`)
        .expect(404);
    });

    it('should return 404 when deleting non-existent user', async () => {
      const testUser = await createTestUser();
      const token = generateJwtToken(testUser.id);

      await request(app)
        .delete('/api/v1/users/non-existent-id')
        .set('Authorization', `Bearer ${token}`)
        .expect(404);
    });
  });
});
```

## DevOps Templates

### 1. Docker Templates

#### Multi-stage Dockerfile for Node.js
```dockerfile
# Dockerfile
# Build stage
FROM node:18-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./
COPY yarn.lock ./

# Install dependencies
RUN yarn install --frozen-lockfile

# Copy source code
COPY . .

# Build application
RUN yarn build

# Production stage
FROM node:18-alpine AS production

# Create non-root user
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nextjs -u 1001

WORKDIR /app

# Copy package files and install production dependencies
COPY package*.json ./
COPY yarn.lock ./
RUN yarn install --frozen-lockfile --production && yarn cache clean

# Copy built application from builder stage
COPY --from=builder --chown=nextjs:nodejs /app/dist ./dist
COPY --from=builder --chown=nextjs:nodejs /app/public ./public

# Set environment variables
ENV NODE_ENV=production
ENV PORT=3000

# Expose port
EXPOSE 3000

# Switch to non-root user
USER nextjs

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Start application
CMD ["node", "dist/server.js"]
```

#### Docker Compose for Development
```yaml
# docker-compose.yml
version: '3.8'

services:
  app:
    build:
      context: .
      target: development
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=development
      - DATABASE_URL=postgresql://user:password@db:5432/appdb
      - REDIS_URL=redis://redis:6379
    volumes:
      - .:/app
      - /app/node_modules
    depends_on:
      - db
      - redis
    networks:
      - app-network

  db:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
    environment:
      - POSTGRES_DB=appdb
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./db/init.sql:/docker-entrypoint-initdb.d/init.sql
    networks:
      - app-network

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    networks:
      - app-network

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - app
    networks:
      - app-network

volumes:
  postgres_data:
  redis_data:

networks:
  app-network:
    driver: bridge
```

### 2. CI/CD Templates

#### GitHub Actions Workflow
```yaml
# .github/workflows/ci-cd.yml
name: CI/CD Pipeline

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  NODE_VERSION: '18'
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'

      - name: Install dependencies
        run: npm ci

      - name: Run linting
        run: npm run lint

      - name: Run type checking
        run: npm run type-check

      - name: Run unit tests
        run: npm run test:unit
        env:
          NODE_ENV: test

      - name: Run integration tests
        run: npm run test:integration
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/test

      - name: Generate test coverage
        run: npm run test:coverage

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          token: ${{ secrets.CODECOV_TOKEN }}

  security:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Run security audit
        run: npm audit --audit-level high

      - name: Run dependency vulnerability scan
        uses: snyk/actions/node@master
        env:
          SNYK_TOKEN: ${{ secrets.SNYK_TOKEN }}

  build:
    needs: [test, security]
    runs-on: ubuntu-latest
    outputs:
      image: ${{ steps.image.outputs.image }}
      digest: ${{ steps.build.outputs.digest }}
    
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=ref,event=branch
            type=ref,event=pr
            type=sha,prefix=commit-

      - name: Build and push Docker image
        id: build
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Output image
        id: image
        run: |
          echo "image=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:commit-${{ github.sha }}" >> $GITHUB_OUTPUT

  deploy-staging:
    if: github.ref == 'refs/heads/develop'
    needs: build
    runs-on: ubuntu-latest
    environment: staging
    
    steps:
      - name: Deploy to staging
        uses: azure/webapps-deploy@v2
        with:
          app-name: ${{ secrets.AZURE_WEBAPP_NAME_STAGING }}
          publish-profile: ${{ secrets.AZURE_WEBAPP_PUBLISH_PROFILE_STAGING }}
          images: ${{ needs.build.outputs.image }}

      - name: Run smoke tests
        run: |
          curl -f ${{ secrets.STAGING_URL }}/health || exit 1
          npm run test:smoke -- --url=${{ secrets.STAGING_URL }}

  deploy-production:
    if: github.ref == 'refs/heads/main'
    needs: build
    runs-on: ubuntu-latest
    environment: production
    
    steps:
      - name: Deploy to production
        uses: azure/webapps-deploy@v2
        with:
          app-name: ${{ secrets.AZURE_WEBAPP_NAME }}
          publish-profile: ${{ secrets.AZURE_WEBAPP_PUBLISH_PROFILE }}
          images: ${{ needs.build.outputs.image }}

      - name: Run production health checks
        run: |
          curl -f ${{ secrets.PRODUCTION_URL }}/health || exit 1
          
      - name: Notify deployment success
        uses: 8398a7/action-slack@v3
        with:
          status: success
          channel: '#deployments'
          text: 'Production deployment successful! 🚀'
        env:
          SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK }}

      - name: Notify deployment failure
        if: failure()
        uses: 8398a7/action-slack@v3
        with:
          status: failure
          channel: '#deployments'
          text: 'Production deployment failed! 🚨 @here'
        env:
          SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK }}
```

## Code Quality Templates

### 1. ESLint Configuration
```json
{
  "extends": [
    "eslint:recommended",
    "@typescript-eslint/recommended",
    "prettier"
  ],
  "parser": "@typescript-eslint/parser",
  "parserOptions": {
    "ecmaVersion": 2022,
    "sourceType": "module",
    "project": "./tsconfig.json"
  },
  "plugins": ["@typescript-eslint", "import", "security"],
  "rules": {
    "@typescript-eslint/no-unused-vars": "error",
    "@typescript-eslint/explicit-function-return-type": "warn",
    "@typescript-eslint/no-explicit-any": "warn",
    "@typescript-eslint/prefer-const": "error",
    "import/order": [
      "error",
      {
        "groups": [
          "builtin",
          "external",
          "internal",
          "parent",
          "sibling",
          "index"
        ],
        "newlines-between": "always",
        "alphabetize": {
          "order": "asc",
          "caseInsensitive": true
        }
      }
    ],
    "security/detect-object-injection": "warn",
    "security/detect-non-literal-regexp": "warn",
    "no-console": "warn",
    "no-debugger": "error",
    "prefer-const": "error",
    "no-var": "error"
  },
  "env": {
    "node": true,
    "es2022": true
  },
  "overrides": [
    {
      "files": ["**/__tests__/**/*", "**/*.test.*"],
      "env": {
        "jest": true
      },
      "rules": {
        "@typescript-eslint/no-explicit-any": "off"
      }
    }
  ]
}
```

These comprehensive templates provide a solid foundation for development teams to maintain consistency, quality, and efficiency across all projects. Each template includes best practices for security, testing, documentation, and maintainability.