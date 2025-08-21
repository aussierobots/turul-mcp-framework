# Code Review Guidelines

## Overview

This document provides comprehensive guidelines for conducting effective code reviews that improve code quality, knowledge sharing, and team collaboration.

## Review Objectives

### Primary Goals
- **Code Quality**: Ensure code meets quality standards and best practices
- **Knowledge Sharing**: Spread knowledge across the team
- **Bug Prevention**: Catch issues before they reach production
- **Consistency**: Maintain consistent coding patterns and architecture
- **Learning**: Provide opportunities for growth and skill development

### Secondary Benefits
- **Documentation**: Review comments serve as additional documentation
- **Team Building**: Foster collaboration and communication
- **Risk Mitigation**: Reduce single points of failure in knowledge

## Review Process

### 1. Pre-Review Checklist (Author)

Before requesting a review, ensure:

- [ ] **Self-review completed**: Review your own code first
- [ ] **Tests pass**: All automated tests are passing
- [ ] **Documentation updated**: README, API docs, comments updated
- [ ] **Breaking changes noted**: Any API changes clearly documented
- [ ] **Performance impact assessed**: Consider performance implications
- [ ] **Security review**: Check for common security issues

### 2. Review Request Best Practices

- **Keep changes small**: Aim for <400 lines of code per review
- **Provide context**: Include clear description of what and why
- **Link to issues**: Reference related tickets or requirements
- **Highlight areas of concern**: Point out complex or risky sections
- **Include testing instructions**: How to test the changes

### 3. Reviewer Guidelines

#### Review Focus Areas

1. **Correctness**
   - Does the code do what it's supposed to do?
   - Are edge cases handled properly?
   - Is error handling comprehensive?

2. **Design and Architecture**
   - Is the approach sound and maintainable?
   - Does it follow established patterns?
   - Are abstractions appropriate?

3. **Performance**
   - Are there obvious performance issues?
   - Is resource usage reasonable?
   - Are algorithms efficient?

4. **Security**
   - Are inputs validated and sanitized?
   - Are authentication/authorization checks present?
   - Is sensitive data handled securely?

5. **Testing**
   - Are tests comprehensive and meaningful?
   - Do tests cover edge cases and error conditions?
   - Are tests maintainable and readable?

6. **Documentation**
   - Is code self-documenting with clear names?
   - Are complex algorithms explained?
   - Is public API documented?

#### Review Techniques

**Code Reading Strategies:**
- Start with the high-level structure
- Understand the data flow
- Identify the main algorithms
- Check error handling paths
- Review test coverage

**Common Anti-patterns to Watch For:**
- God classes/functions (too much responsibility)
- Deep nesting (consider early returns)
- Magic numbers/strings (use constants)
- Duplicated code (extract common functionality)
- Poor naming (unclear or misleading names)

## Language-Specific Guidelines

### Rust Code Reviews

**Focus Areas:**
- **Memory Safety**: Check for proper lifetime management
- **Error Handling**: Ensure Result/Option types are used appropriately
- **Performance**: Look for unnecessary allocations or clones
- **Idioms**: Verify idiomatic Rust patterns are used
- **Unsafe Code**: Scrutinize any unsafe blocks carefully

**Common Issues:**
- Unnecessary `.clone()` calls
- Missing error propagation with `?` operator
- Improper use of `unwrap()` vs proper error handling
- Inefficient string handling
- Missing documentation for public APIs

### Python Code Reviews

**Focus Areas:**
- **Type Safety**: Check type hints and mypy compliance
- **Performance**: Identify potential bottlenecks
- **Style**: Ensure PEP 8 compliance
- **Testing**: Verify pytest best practices
- **Dependencies**: Check for unnecessary or outdated packages

**Common Issues:**
- Missing or incorrect type hints
- Inefficient list comprehensions or loops
- Mutable default arguments
- Bare except clauses
- Missing docstrings for public functions

### JavaScript/TypeScript Code Reviews

**Focus Areas:**
- **Type Safety**: TypeScript usage and type definitions
- **Async Handling**: Proper promise/async-await usage
- **Performance**: Bundle size and runtime performance
- **Security**: XSS, injection vulnerabilities
- **Browser Compatibility**: Cross-browser support

**Common Issues:**
- Unhandled promise rejections
- Memory leaks in event listeners
- Inefficient DOM manipulation
- Missing error boundaries in React
- Prototype pollution vulnerabilities

## Review Communication

### Effective Feedback

**Constructive Comments:**
- Be specific about the issue
- Explain the reasoning behind suggestions
- Provide examples or alternatives
- Acknowledge good code and improvements

**Comment Categories:**
- **Must Fix**: Critical issues that block merge
- **Should Fix**: Important improvements that should be addressed
- **Consider**: Suggestions for improvement or discussion
- **Nit**: Minor style or preference issues
- **Question**: Requests for clarification

**Example Comments:**

**Good:**
```
Consider: This loop could be more efficient using a HashMap lookup 
instead of linear search. With the current approach, time complexity 
is O(n²) which might become a bottleneck with larger datasets.

Suggested approach:
let lookup: HashMap<_, _> = items.iter().enumerate().collect();
```

**Avoid:**
```
This is slow.
```

### Handling Disagreements

1. **Focus on the code, not the person**
2. **Provide technical justification**
3. **Consider multiple valid approaches**
4. **Escalate if needed** (tech lead, architecture review)
5. **Document decisions** for future reference

## Review Tools and Automation

### Automated Checks

Leverage automation for:
- **Code formatting** (rustfmt, black, prettier)
- **Linting** (clippy, pylint, eslint)
- **Type checking** (cargo check, mypy, tsc)
- **Security scanning** (cargo audit, bandit, npm audit)
- **Test coverage** (tarpaulin, coverage.py, jest)

### Review Platform Features

Use platform features effectively:
- **Draft reviews** for work-in-progress feedback
- **Review requests** to specific domain experts
- **Code suggestions** for simple fixes
- **Approval requirements** for critical paths
- **Status checks** integration

## Team Standards

### Review Assignments

- **Round-robin**: Distribute review load evenly
- **Domain expertise**: Assign based on code area knowledge
- **Learning opportunities**: Junior developers review senior code
- **Cross-team reviews**: For shared components or APIs

### Review SLA

- **Initial response**: Within 4 hours during business hours
- **Complete review**: Within 24 hours for normal changes
- **Urgent reviews**: Within 2 hours for hotfixes
- **Large reviews**: Schedule dedicated time, break into smaller parts

### Escalation Process

1. **Reviewer concerns**: Discuss with author first
2. **Persistent disagreements**: Involve tech lead
3. **Architectural decisions**: Architecture review board
4. **Cross-team impacts**: Include relevant stakeholders

## Metrics and Improvement

### Review Metrics

Track and analyze:
- **Review turnaround time**
- **Number of review rounds**
- **Defect detection rate**
- **Review coverage percentage**
- **Time to approval**

### Continuous Improvement

Regular retrospectives on:
- Review process effectiveness
- Common issues and patterns
- Training needs and opportunities
- Tool and automation improvements
- Team communication and collaboration

## Emergency Procedures

### Hotfix Reviews

For critical production issues:
- **Expedited process**: Immediate review by senior developer
- **Limited scope**: Focus on fix correctness and safety
- **Follow-up review**: Comprehensive review post-deployment
- **Documentation**: Document lessons learned

### Security Issues

For security-related changes:
- **Security expert review**: Always include security-focused reviewer
- **Threat modeling**: Consider attack vectors and mitigations
- **Compliance check**: Ensure regulatory requirements are met
- **Audit trail**: Maintain detailed review records

This comprehensive review process ensures code quality while fostering team collaboration and continuous learning.