# Ember Engine
Ember is a graphics and game engine written in Rust. 

### Design Goals
A [fast](#fast), flexible, and complete library crate which facilitates the development of 2d and 3d applications, including but not limited to simulations, games, and animations. 

#### Fast
[ECS](https://en.wikipedia.org/wiki/Entity_component_system) architecture is faster and can more inherently take advantage of several cpu cores than OOP.
No scripting engine; users import the engine into their own binary crates.
Rust is also very cool and fast.

#### Flexible
Both high and low level APIs should exist. In other words, all of the following users should feel welcome here.

_**The Game Dev**_
Uses the well-documented top-level Ember API to access the features described in the [completeness](#complete) section.

_**The Graphics Guru**_
Uses the top-level API in addition to writing their own WGSL shaders and integrating them with custom shader nodes and render graphs. 

#### Complete

