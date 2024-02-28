package org.archipel.generator;

import java.util.Random;

public class Generator
{
    private static final float INPUT_FACTOR = 1.0181268882175227f;
    private final long seed;
    private final ImprovedPerlinNoise perlinNoise;

    public Generator(long seed)
    {
        this.seed = seed;
        this.perlinNoise = new ImprovedPerlinNoise(new Random(this.seed));
    }

    public int getBlock(int x, int y, int z)
    {
        final var noise = this.perlinNoise.fractalBrownianMotion(x * INPUT_FACTOR, z * INPUT_FACTOR, 8);
        final var surfaceLevel = noise * 35;
        if(y < surfaceLevel)
            return 2;
        else return 0;
    }

    public long getSeed()
    {
        return this.seed;
    }
}
