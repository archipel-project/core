package org.archipel.generator;

public class Generator
{
    private final long seed;

    public Generator(long seed) {
        this.seed = seed;
    }

    public int getBlock(int x, int y, int z)
    {
        return y < 0 ? 1 : 0;
    }
}
