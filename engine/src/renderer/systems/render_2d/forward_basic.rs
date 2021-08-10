// This should render an unlimited number of Render2DBasic objects.
// I needed dynamic uniforms because I wanted to use a single, larger uniform buffer for all the objects
// This happened because I was creating one uniform group per node/pipeline.
// In the future, I need a way to create new uniform buffers per-entity; perhaps using the uniform group builder,
// although this is stored in the node builder atm.
