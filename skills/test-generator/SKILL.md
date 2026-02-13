---
name: test-generator
description: Generates comprehensive test suites for TypeScript, JavaScript, Python, Rust, and Go code. Use when creating tests or improving test coverage.
---

# Test Suite Generator

Automatically generates comprehensive test suites following best practices and AAA pattern.

## Activation

Use when:
- User asks to "generate tests"
- User says "write tests for [file/function]"
- User requests "improve test coverage"
- User asks "how should I test this?"

## Instructions

1. Analyze the code to test:
   - Identify public functions/methods
   - Understand input/output types
   - Note side effects and dependencies
2. Generate test structure:
   - Group related tests in describe blocks
   - Cover happy path, edge cases, error conditions
   - Mock external dependencies
3. Follow language-specific patterns:
   - TypeScript/JavaScript: Jest or Vitest
   - Python: pytest
   - Rust: cargo test
   - Go: testing package

## Test Structure (AAA Pattern)

```typescript
describe('FunctionName', () => {
  // Setup helpers
  const setup = () => ({
    // Common test data and mocks
  });

  describe('happy path', () => {
    it('should [expected behavior]', () => {
      // Arrange
      const { input } = setup();

      // Act
      const result = functionName(input);

      // Assert
      expect(result).toEqual(expected);
    });
  });

  describe('edge cases', () => {
    it('should handle empty input', () => { });
    it('should handle null/undefined', () => { });
    it('should handle boundary values', () => { });
  });

  describe('error conditions', () => {
    it('should throw on invalid input', () => {
      expect(() => functionName(invalid)).toThrow(ValidationError);
    });
  });
});
```

## Language-Specific Patterns

### TypeScript/JavaScript (Jest/Vitest)

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { UserService } from './user-service';

describe('UserService', () => {
  let service: UserService;
  let mockDb: any;

  beforeEach(() => {
    mockDb = {
      query: vi.fn(),
      insert: vi.fn(),
    };
    service = new UserService(mockDb);
  });

  describe('createUser', () => {
    it('should create a new user', async () => {
      const userData = { name: 'John', email: 'john@example.com' };
      mockDb.insert.mockResolvedValue({ id: '123', ...userData });

      const result = await service.createUser(userData);

      expect(result).toEqual({ id: '123', ...userData });
      expect(mockDb.insert).toHaveBeenCalledWith('users', userData);
    });

    it('should throw on duplicate email', async () => {
      mockDb.insert.mockRejectedValue(new Error('duplicate key'));

      await expect(service.createUser({ name: 'John', email: 'john@example.com' }))
        .rejects
        .toThrow('User already exists');
    });
  });
});
```

### Python (pytest)

```python
import pytest
from user_service import UserService, ValidationError

class TestUserService:
    @pytest.fixture
    def service(self, mocker):
        mock_db = mocker.Mock()
        return UserService(mock_db), mock_db

    def test_create_user_success(self, service):
        user_service, mock_db = service
        user_data = {"name": "John", "email": "john@example.com"}
        mock_db.insert.return_value = {"id": "123", **user_data}

        result = user_service.create_user(user_data)

        assert result == {"id": "123", **user_data}
        mock_db.insert.assert_called_once_with("users", user_data)

    def test_create_user_duplicate_email(self, service):
        user_service, mock_db = service
        mock_db.insert.side_effect = ValueError("duplicate key")

        with pytest.raises(ValidationError, match="User already exists"):
            user_service.create_user({"name": "John", "email": "john@example.com"})
```

### Rust (cargo test)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_success() {
        let service = UserService::new();
        let user_data = UserData {
            name: "John".to_string(),
            email: "john@example.com".to_string(),
        };

        let result = service.create_user(user_data);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.name, "John");
    }

    #[test]
    fn test_create_user_invalid_email() {
        let service = UserService::new();
        let user_data = UserData {
            name: "John".to_string(),
            email: "invalid-email".to_string(),
        };

        let result = service.create_user(user_data);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Invalid email format");
    }
}
```

### Go (testing package)

```go
package service

import (
    "testing"
    "github.com/stretchr/testify/assert"
    "github.com/stretchr/testify/mock"
)

type MockDB struct {
    mock.Mock
}

func (m *MockDB) Insert(table string, data interface{}) error {
    args := m.Called(table, data)
    return args.Error(0)
}

func TestCreateUser(t *testing.T) {
    t.Run("success", func(t *testing.T) {
        mockDB := new(MockDB)
        service := NewUserService(mockDB)

        userData := UserData{Name: "John", Email: "john@example.com"}
        mockDB.On("Insert", "users", userData).Return(nil)

        result, err := service.CreateUser(userData)

        assert.NoError(t, err)
        assert.Equal(t, "John", result.Name)
        mockDB.AssertExpectations(t)
    })

    t.Run("duplicate email", func(t *testing.T) {
        mockDB := new(MockDB)
        service := NewUserService(mockDB)

        userData := UserData{Name: "John", Email: "john@example.com"}
        mockDB.On("Insert", "users", userData).Return(errors.New("duplicate key"))

        _, err := service.CreateUser(userData)

        assert.Error(t, err)
        assert.Contains(t, err.Error(), "already exists")
    })
}
```

## Coverage Requirements

- **Minimum 80% line coverage** for new code
- **100% branch coverage** for critical paths
- **All edge cases** tested (null, empty, boundary values)
- **Error conditions** tested (exceptions, errors)
- **Integration points** mocked appropriately

## Best Practices

- **Fast tests:** Unit tests should run in < 100ms each
- **Isolated:** No dependencies on external services
- **Deterministic:** Same input always produces same output
- **Readable:** Test names describe what they test
- **Maintainable:** Easy to update when code changes
- **Focused:** One assertion per test when possible

