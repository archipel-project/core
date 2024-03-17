package org.archipel.generator;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Random;

public class ImprovedPerlinNoise
{
    private final int[] permutations;

    public ImprovedPerlinNoise(Random random)
    {
        final List<Integer> list = new ArrayList<>(256);
        for (int i = 0; i < 256; i++)
            list.add(i);

        Collections.shuffle(list, random);

        this.permutations = new int[512];
        for (int i = 0; i < 256; i++)
            this.permutations[i] = this.permutations[i + 256] = list.get(i);
    }

    public float fade(float t)
    {
        return ((6 * t - 15) * t + 10) * t * t * t;
    }

    public float lerp(float t, float a, float b)
    {
        return a + t * (b - a);
    }

    private static final Vec2 NEG_POS = new Vec2(-1.0f, 1.0f);
    private static final Vec2 NEG_NEG = new Vec2(-1.0f, -1.0f);
    private static final Vec2 POS_NEG = new Vec2(1.0f, -1.0f);

    public Vec2 getConstantVector(int val)
    {
	    final var hash = val & 3;
        if(hash == 0)
            return Vec2.ONE;
        else if(hash == 1)
            return NEG_POS;
        else if(hash == 2)
            return NEG_NEG;
        else
            return POS_NEG;
    }

    public float noise2d(float x, float y)
    {
        final int X = Mth.floor(x) & 255;
        final int Y = Mth.floor(y) & 255;

        final float xf = x - Mth.floor(x);
        final float yf = y - Mth.floor(y);
    
        final var topRight = new Vec2(xf - 1.0f, yf - 1.0f);
        final var topLeft = new Vec2(xf, yf - 1.0f);
        final var bottomRight = new Vec2(xf - 1.0f, yf);
        final var bottomLeft = new Vec2(xf, yf);
    
        final var valueTopRight = this.permutations[this.permutations[X+1]+Y+1];
        final var valueTopLeft = this.permutations[this.permutations[X]+Y+1];
        final var valueBottomRight = this.permutations[this.permutations[X+1]+Y];
        final var valueBottomLeft = this.permutations[this.permutations[X]+Y];
        
        final var dotTopRight = topRight.dot(this.getConstantVector(valueTopRight));
        final var dotTopLeft = topLeft.dot(this.getConstantVector(valueTopLeft));
        final var dotBottomRight = bottomRight.dot(this.getConstantVector(valueBottomRight));
        final var dotBottomLeft = bottomLeft.dot(this.getConstantVector(valueBottomLeft));
        
        final var u = this.fade(xf);
        final var v = this.fade(yf);

        return this.lerp(u, this.lerp(v, dotBottomLeft, dotTopLeft), this.lerp(v, dotBottomRight, dotTopRight));
    }

    public float fractalBrownianMotion(float x, float y, int numOctaves)
    {
        float result = 0.0f;
        float amplitude = 1.0f;
        float frequency = 0.005f;

        for (int octave = 0; octave < numOctaves; octave++)
        {
		    final float n = amplitude * this.noise2d(x * frequency, y * frequency);
            result += n;

            amplitude *= 0.5f;
            frequency *= 2.0f;
        }

        return result;
    }
}
