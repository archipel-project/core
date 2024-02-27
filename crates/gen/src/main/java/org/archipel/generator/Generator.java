package org.archipel.generator;

public class Generator
{
    private final long seed;
    private final ImprovedPerlinNoise perlinNoise = new ImprovedPerlinNoise();

    public Generator(long seed)
    {
        this.seed = seed;
    }

    public int getBlock(int x, int y, int z)
    {
        final var noise = this.perlinNoise.fractalBrownianMotion(x, z, 8);
        final var surfaceLevel = y + noise * 20;
        if(y < surfaceLevel)
            return 1;
        else return 0;
    }
}
