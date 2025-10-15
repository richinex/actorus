No simulation or simulating of any kind
No use of emojis
## Strictly Follow Modularization Approach Detailed Below
1. Information Hiding vs. Flow-Chart Decomposition

Traditional approach: Modules follow processing steps (input → process → output)
Parnas approach: Modules hide design decisions that are likely to change
The processing flow should NOT dictate module boundaries

2. Benefits of Information Hiding

Changeability: Changes are localized to single modules
Independent Development: Teams can work on modules with minimal coordination
Comprehensibility: Each module can be understood in isolation

3. What to Hide in Modules

Data structures and their internal organization
Algorithms and their implementation details
Hardware dependencies and system interfaces
Input/output formats and protocols
Character encodings and data representations

4. The Cost of Poor Modularization
In the flow-chart approach, seemingly simple changes require modifications to multiple modules:

Changing storage format affects ALL modules
Changing algorithms affects multiple dependent modules
Adding new features requires understanding the entire system

This demonstrates why information hiding is the superior criterion for modularization - it creates systems that are more maintainable, flexible, and comprehensible. Each module becomes a "black box" that can be modified or replaced without affecting others, as long as its interface remains stable.
The examples show how this principle, introduced in 1972, remains fundamental to modern software architecture and is the foundation for concepts like encapsulation, interfaces, and microservices.
for instance
/*
Information Hiding Modularization annotations:
1 Internal storage format hidden from other modules
2 Parsing logic hidden - can change without affecting others
3 Word access interface hides internal representation
4 Word count interface hides internal structure
5 Shift storage format completely hidden from other modules
6 Shift generation algorithm can be changed independently
7 Uses LineStorage interface - doesn't know internal format
8 Provides abstract interface to other modules
9 Sorting algorithm and data structure hidden from others
10 Index initialization strategy is internal implementation detail
11 Sorting algorithm can be swapped without affecting other modules
12 Provides sorted results through clean interface
13 Output formatting uses only public interfaces
14 Completely different internal representation
15 Dictionary compression is internal implementation detail
16 Same interface, different internal mechanism
17 Standard pipeline using LineStorage interface
18 Only this line changes - polymorphism in action!
19 All other modules work unchanged with new storage

Key Benefits:
- Each module hides design decisions from others
- Changes to internal implementations don't affect other modules
- New implementations can be swapped in easily
- Modules can be developed and tested independently
- System is much more maintainable and flexible
*/
## Consult the Rust docs when in doubt

## Always make a plan
