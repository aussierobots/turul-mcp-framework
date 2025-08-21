# Customer Onboarding Reference Data

This document contains reference data used throughout the customer onboarding and data collection platform. This data is used to populate choice fields, validate inputs, and provide context for form generation.

## Geographic Data

### US States and Territories
```
Alabama, Alaska, Arizona, Arkansas, California, Colorado, Connecticut, Delaware, Florida, Georgia, Hawaii, Idaho, Illinois, Indiana, Iowa, Kansas, Kentucky, Louisiana, Maine, Maryland, Massachusetts, Michigan, Minnesota, Mississippi, Missouri, Montana, Nebraska, Nevada, New Hampshire, New Jersey, New Mexico, New York, North Carolina, North Dakota, Ohio, Oklahoma, Oregon, Pennsylvania, Rhode Island, South Carolina, South Dakota, Tennessee, Texas, Utah, Vermont, Virginia, Washington, West Virginia, Wisconsin, Wyoming, American Samoa, District of Columbia, Federated States of Micronesia, Guam, Marshall Islands, Northern Mariana Islands, Palau, Puerto Rico, Virgin Islands
```

### Canadian Provinces and Territories
```
Alberta, British Columbia, Manitoba, New Brunswick, Newfoundland and Labrador, Northwest Territories, Nova Scotia, Nunavut, Ontario, Prince Edward Island, Quebec, Saskatchewan, Yukon
```

### Supported Countries
```
United States, Canada, United Kingdom, Australia, Germany, France, Italy, Spain, Netherlands, Belgium, Switzerland, Austria, Sweden, Norway, Denmark, Finland, Ireland, Portugal, Japan, South Korea, Singapore, Hong Kong, New Zealand, Brazil, Mexico, India, Israel, South Africa
```

### World Time Zones (Major Cities)
```
Pacific/Honolulu (UTC-10) - Hawaii
America/Anchorage (UTC-9) - Alaska
America/Los_Angeles (UTC-8) - Pacific Time
America/Denver (UTC-7) - Mountain Time
America/Chicago (UTC-6) - Central Time
America/New_York (UTC-5) - Eastern Time
America/Halifax (UTC-4) - Atlantic Time
America/St_Johns (UTC-3:30) - Newfoundland Time
America/Sao_Paulo (UTC-3) - Brazil
UTC (UTC+0) - Greenwich Mean Time
Europe/London (UTC+0/+1) - UK Time
Europe/Paris (UTC+1) - Central European Time
Europe/Helsinki (UTC+2) - Eastern European Time
Europe/Moscow (UTC+3) - Moscow Time
Asia/Dubai (UTC+4) - Gulf Time
Asia/Kolkata (UTC+5:30) - India Time
Asia/Shanghai (UTC+8) - China Time
Asia/Tokyo (UTC+9) - Japan Time
Australia/Sydney (UTC+10/+11) - Australian Eastern Time
Pacific/Auckland (UTC+12/+13) - New Zealand Time
```

## Industry Classifications (NAICS)

### Major Industry Sectors
```
Agriculture, Forestry, Fishing and Hunting
Mining, Quarrying, and Oil and Gas Extraction
Utilities
Construction
Manufacturing
Wholesale Trade
Retail Trade
Transportation and Warehousing
Information
Finance and Insurance
Real Estate and Rental and Leasing
Professional, Scientific, and Technical Services
Management of Companies and Enterprises
Administrative and Support and Waste Management Services
Educational Services
Health Care and Social Assistance
Arts, Entertainment, and Recreation
Accommodation and Food Services
Other Services (except Public Administration)
Public Administration
```

### Technology Subsectors
```
Software Publishers
Computer Systems Design and Related Services
Data Processing, Hosting, and Related Services
Internet Publishing and Broadcasting
Telecommunications
Electronic Shopping and Mail-Order Houses
Computer and Electronic Product Manufacturing
Semiconductor and Other Electronic Component Manufacturing
```

### Financial Services Subsectors
```
Commercial Banking
Investment Banking and Securities Dealing
Asset Management and Custody Activities
Insurance Carriers
Credit Intermediation
Real Estate Credit
Consumer Lending
Payment Processing Services
Financial Transaction Processing
Cryptocurrency and Digital Asset Services
```

## Compliance and Regulatory Information

### Regulatory Frameworks
```
GDPR - General Data Protection Regulation (EU)
CCPA - California Consumer Privacy Act
CPRA - California Privacy Rights Act
PIPEDA - Personal Information Protection and Electronic Documents Act (Canada)
LGPD - Lei Geral de Proteção de Dados (Brazil)
PDPA - Personal Data Protection Act (Singapore)
Privacy Act 1988 (Australia)
POPI Act - Protection of Personal Information Act (South Africa)
```

### Industry-Specific Regulations
```
HIPAA - Health Insurance Portability and Accountability Act
PCI DSS - Payment Card Industry Data Security Standard
SOX - Sarbanes-Oxley Act
GLBA - Gramm-Leach-Bliley Act
FERPA - Family Educational Rights and Privacy Act
COPPA - Children's Online Privacy Protection Act
CAN-SPAM Act
TCPA - Telephone Consumer Protection Act
GDPR-K (Kids) - Special protections for children
```

## Security and Authentication

### Password Security Requirements
```
Minimum Length: 12 characters
Character Requirements:
- At least 1 uppercase letter (A-Z)
- At least 1 lowercase letter (a-z)
- At least 1 number (0-9)
- At least 1 special character (!@#$%^&*()_+-=[]{}|;:,.<>?)

Forbidden Patterns:
- Common passwords (password, 123456, qwerty, admin, login)
- Personal information (name, email, phone)
- Keyboard patterns (asdfgh, 123456789)
- Dictionary words
- Repeated characters (aaaaaa, 111111)

Entropy Requirements:
- Minimum entropy score: 50 bits
- Maximum consecutive identical characters: 2
- Maximum keyboard sequence length: 3
```

### Two-Factor Authentication Methods
```
Authenticator Apps:
- Google Authenticator
- Microsoft Authenticator
- Authy
- 1Password
- LastPass Authenticator

Hardware Tokens:
- YubiKey
- RSA SecurID
- FIDO2 Security Keys
- Smart Cards

Backup Methods:
- SMS (not recommended for high-security)
- Email backup codes
- Recovery codes (one-time use)
- Trusted device verification
```

## Document Types and Requirements

### Individual Identity Documents
```
Primary Documents (Photo ID):
- Driver's License
- Passport
- State-issued ID Card
- Military ID
- Tribal ID

Secondary Documents (Address Verification):
- Utility Bill (within 90 days)
- Bank Statement (within 90 days)
- Credit Card Statement (within 90 days)
- Insurance Statement
- Government Correspondence
- Lease Agreement
```

### Business Documents
```
Formation Documents:
- Articles of Incorporation
- Articles of Organization (LLC)
- Partnership Agreement
- DBA Certificate
- Business License

Tax Documents:
- EIN Letter from IRS
- State Tax ID Certificate
- Sales Tax Permit
- Professional License

Financial Documents:
- Business Bank Account Statements
- Financial Statements
- Tax Returns (Business)
- Proof of Business Insurance
```

## Communication Preferences

### Notification Channels
```
Email:
- Immediate delivery
- Rich formatting support
- Attachment capability
- Archive and search
- Cost-effective for bulk
- GDPR compliant unsubscribe

SMS/Text:
- High open rates (98%)
- Character limit (160)
- Delivery confirmation
- Opt-in required
- Carrier fees apply
- Time zone considerations

Push Notifications:
- Real-time delivery
- Rich media support
- Action buttons
- Badge counts
- Device-specific targeting
- Battery impact considerations

In-App Notifications:
- Context-aware
- Rich interactions
- No external dependencies
- User must be logged in
- Immediate feedback
```

### Marketing Communication Types
```
Transactional:
- Account creation confirmations
- Password reset notifications
- Transaction confirmations
- Statement availability
- Security alerts
- Service updates

Promotional:
- New feature announcements
- Special offers and discounts
- Event invitations
- Product recommendations
- Seasonal campaigns
- Referral programs

Educational:
- How-to guides and tutorials
- Best practice tips
- Industry insights
- Webinar invitations
- Resource downloads
- Newsletter content
```

## Accessibility Standards

### WCAG 2.1 Level AA Compliance
```
Perceivable:
- Color contrast ratio ≥ 4.5:1
- Alternative text for images
- Captions for videos
- Resizable text up to 200%

Operable:
- Keyboard accessible
- No seizure-inducing content
- Sufficient time to read
- Clear navigation

Understandable:
- Readable text
- Predictable functionality
- Input assistance
- Error identification

Robust:
- Compatible with assistive technologies
- Valid HTML markup
- Future-proof code
```

### Assistive Technology Support
```
Screen Readers:
- JAWS (Windows)
- NVDA (Windows, free)
- VoiceOver (macOS/iOS)
- TalkBack (Android)
- Dragon NaturallySpeaking (Voice control)

Mobility Assistive Devices:
- Switch controls
- Eye-tracking systems
- Mouth stick/head pointer
- Voice control software
- One-handed keyboards

Cognitive Assistive Tools:
- Reading assistants
- Memory aids
- Focus enhancement tools
- Simplified interfaces
- Time management tools
```

## Data Validation Services

### Third-Party Validation APIs
```
Email Validation:
- Mailgun Email Validation
- ZeroBounce
- Hunter.io
- EmailListVerify
- Kickbox

Address Validation:
- USPS Address Validation
- Google Places API
- Here Geocoding API
- Melissa Global Address
- SmartyStreets

Phone Validation:
- Twilio Lookup API
- Numverify
- Phone Number Validation API
- AbstractAPI Phone Validation
- VoilaAPI

Identity Verification:
- Jumio
- Onfido
- Shufti Pro
- Veriff
- IDnow
```

### Data Quality Metrics
```
Completeness:
- Percentage of required fields filled
- Critical field completion rate
- Optional field utilization

Accuracy:
- Email deliverability rate
- Address validation score
- Phone number connectivity
- Document authenticity score

Consistency:
- Cross-field validation success
- Format standardization compliance
- Duplicate detection accuracy
- Reference data matching

Timeliness:
- Data freshness indicators
- Last update timestamps
- Verification recency
- Document expiration tracking
```

## Error Messages and User Experience

### User-Friendly Error Messages
```
Generic Errors:
- "This field is required" → "Please fill in this field"
- "Invalid input" → "Please check your entry and try again"
- "Format error" → "Please use the correct format"

Specific Errors:
- Email: "Please enter a valid email address (e.g., you@example.com)"
- Phone: "Please include your country code (e.g., +1-555-123-4567)"
- Date: "Please enter a valid date (MM/DD/YYYY)"
- Password: "Password must be at least 12 characters with uppercase, lowercase, numbers, and symbols"
- File Upload: "File must be a PDF, JPG, or PNG under 10MB"
- Age: "You must be at least 18 years old to create an account"
```

### Progressive Enhancement Messages
```
Loading States:
- "Validating email address..."
- "Checking document..."
- "Saving your information..."
- "Processing form..."

Success States:
- "Email verified successfully ✓"
- "Document uploaded ✓"
- "Information saved ✓"
- "Account created successfully!"

Warning States:
- "Some information couldn't be verified"
- "Document quality is low, please retake"
- "This email is already registered"
- "Password strength: Medium (consider adding more characters)"
```